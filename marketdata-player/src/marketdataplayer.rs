use anyhow::{Ok, Result};
use clickhouse::Client;
use fpdec::Decimal;
use statrs::distribution::{ContinuousCDF, FisherSnedecor};
use std::str::FromStr;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use crate::{dataprovider::DataProvider, orderbook::Orderbook, Event};

pub struct MarketdataPlayer {
    last_update_id: Option<u64>,
    dataprovider: DataProvider,
    orderbook: Orderbook,
    quantity_execution: Decimal,
    model: Model, // create a superstructure simulator
}

impl MarketdataPlayer {
    pub async fn new(
        product: String,
        tablename: String,
        start_timestamp: String,
        quantity_execution: Decimal,
    ) -> Self {
        let client = Client::default()
            .with_url("http://127.0.1.1:8123")
            .with_user("default")
            .with_database("default")
            .with_compression(clickhouse::Compression::None);
        Self {
            last_update_id: None,
            dataprovider: DataProvider::new(client, product, tablename, &start_timestamp),
            orderbook: Orderbook::default(),
            quantity_execution: quantity_execution,
            model: Model::reinit(0.0),
        }
    }
    pub async fn play(&mut self) -> Result<()> {
        while let Some(event) = self.dataprovider.next().await {
            if event.event_type == "snapshot" {
                self.last_update_id = event.id1;
                self.orderbook.update(event)?;
                break;
            }
        }
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(format!("{}.txt", self.dataprovider.product()))
            .await?;
        file.write_all(b"Best player price | Best model price lower | Best model price upper | Real price | Delta execution | Num of obs").await?;
        let mut last_event: Option<Event> = None;
        let mut best_player_total_price = Decimal::ZERO;
        let mut best_model_price_lower = 0.0;
        let mut best_model_price_upper = 0.0;
        let mut real_total_price = Decimal::ZERO;
        let mut delta_execution = 0;
        let mut num_of_obs = 0.0;
        let mut prev_pbest = Decimal::ZERO;
        let quantity_execution = f64::from_str(&self.quantity_execution.to_string()).unwrap();
        while let Some(event) = self.dataprovider.next().await {
            let cur_event = event.clone();
            match event.event_type.as_str() {
                "snapshot" => {
                    if self.last_update_id == event.id1 {
                        self.orderbook.update(event)?;
                    }
                    if let Some(last_event) = last_event {
                        if last_event.event_type == "depth" {
                            self.model = Model::reinit(self.orderbook.pbest());
                        }
                    }
                }
                "depth" => {
                    if let Some(last_event) = last_event {
                        if last_event.event_type == "trade" {
                            best_player_total_price = self
                                .orderbook
                                .best_total_price(self.quantity_execution.clone())
                                .unwrap();
                            let best_price = self.model.get_best_price(0.95);
                            best_model_price_lower =
                                (best_price.0 + self.model.last_pbest) * quantity_execution;
                            best_model_price_upper =
                                (best_price.1 + self.model.last_pbest) * quantity_execution;
                            num_of_obs = best_price.2;
                            delta_execution = event.venue_timestamp - last_event.venue_timestamp;
                        } else if last_event.event_type == "snapshot" {
                            self.model = Model::reinit(self.orderbook.pbest());
                        }
                    }
                    self.orderbook.update(event)?;
                }
                "trade" => {
                    if let Some(last_event) = last_event {
                        if last_event.event_type == "depth" {
                            real_total_price = self
                                .orderbook
                                .best_total_price(self.quantity_execution.clone())
                                .unwrap();
                            if !best_model_price_upper.is_nan() && prev_pbest != real_total_price {
                                let res = format!(
                                    "{} {} {} {} {} {}",
                                    best_player_total_price,
                                    best_model_price_lower,
                                    best_model_price_upper,
                                    real_total_price,
                                    delta_execution,
                                    num_of_obs,
                                );
                                file.write_all(res.as_bytes()).await?;
                                prev_pbest = real_total_price;
                            }
                            self.model = Model::reinit(self.orderbook.pbest());
                        } else if last_event.event_type == "snapshot" {
                            self.model = Model::reinit(self.orderbook.pbest());
                        }
                        let delta_t = event.venue_timestamp - last_event.venue_timestamp;
                        self.model.update(delta_t, event.price.clone())?;
                    }
                    self.orderbook.handle_trade(event)?;
                }
                _ => {}
            }
            last_event = Some(cur_event);
        }
        Ok(())
    }
}

pub struct Model {
    last_pbest: f64,
    time_interval: Vec<f64>,
    price_shift: Vec<f64>,
}

impl Model {
    /// Создание нового экземпляра модели с инициализацией pbest
    pub fn reinit(pbest: f64) -> Self {
        Self {
            last_pbest: pbest,
            time_interval: Vec::new(),
            price_shift: Vec::new(),
        }
    }

    /// Обновление модели: добавление нового времени и изменения цены
    pub fn update(&mut self, delta_t: i64, price: String) -> Result<()> {
        self.time_interval.push((delta_t as f64).ln());
        let price = f64::from_str(&price)?;
        let price_shift = (self.last_pbest - price).abs().sqrt();
        self.price_shift.push(price_shift);
        Ok(())
    }

    /// Расчет доверительного интервала
    pub fn get_best_price(&self, p: f64) -> (f64, f64, f64) {
        let num_of_obs = self.time_interval.len() as f64;
        if num_of_obs < 3.0 {
            // Требуется минимум три наблюдения для статистики Хотеллинга
            return (0.0, 0.0, num_of_obs);
        }

        let time_mean: f64 = self.time_interval.iter().sum::<f64>() / num_of_obs;
        let price_mean: f64 = self.price_shift.iter().sum::<f64>() / num_of_obs;

        let mut variance_time = 0.0;
        let mut variance_price = 0.0;
        let mut covariance_time_price = 0.0;

        for i in 0..self.time_interval.len() {
            let time_diff = self.time_interval[i] - time_mean;
            let price_diff = self.price_shift[i] - price_mean;
            variance_time += time_diff.powi(2);
            variance_price += price_diff.powi(2);
            covariance_time_price += time_diff * price_diff;
        }
        let covariance_coef = variance_time.sqrt()
            / ((variance_time * variance_price - covariance_time_price.powi(2)).sqrt());

        let n = num_of_obs - 2.0;
        if n <= 0.0 {
            return (0.0, 0.0, num_of_obs);
        }
        let f_dist = FisherSnedecor::new(2.0, n).unwrap();
        let hotelling_stat =
            (2.0 * (num_of_obs - 1.0) / (num_of_obs * n) * f_dist.inverse_cdf(p)).sqrt();

        let lower_bound = (price_mean - hotelling_stat / covariance_coef).powi(2);
        let upper_bound = (price_mean + hotelling_stat / covariance_coef).powi(2);

        (lower_bound, upper_bound, num_of_obs)
    }
}
