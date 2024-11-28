mod orderbook;

use anyhow::{Ok, Result};
use chrono::{Duration, NaiveDateTime, Timelike};
use clickhouse::{Client, Row};
use fpdec::Decimal;
use orderbook::{Orderbook, Price};
use serde::Deserialize;
use statrs::distribution::{ContinuousCDF, FisherSnedecor};
use std::{str::FromStr, vec::IntoIter};

#[derive(Row, Deserialize, Debug, Clone)]
pub struct Event {
    local_unique_id: i64,
    venue_timestamp: i64,
    gate_timestamp: i64,
    event_type: String,
    product: String,
    id1: Option<u64>,
    id2: Option<u64>,
    ask_not_bid: Option<bool>,
    buy_not_sell: Option<bool>,
    price: String,
    quantity: String,
}

pub struct DataProvider {
    client: Client,
    product: String,
    tablename: String,
    current_timestamp: NaiveDateTime,
    buffer: IntoIter<Event>,
}

impl DataProvider {
    pub fn new(client: Client, product: String, tablename: String, start_timestamp: &str) -> Self {
        let current_timestamp =
            NaiveDateTime::parse_from_str(start_timestamp, "%Y-%m-%d %H:%M:%S%.f")
                .expect("Failed to parse timestamp");
        Self {
            client,
            product,
            tablename,
            current_timestamp,
            buffer: Vec::new().into_iter(),
        }
    }
    pub async fn load_data(&mut self) -> Option<()> {
        let next_timestamp = self.current_timestamp + Duration::minutes(5);
        let query = format!(
            "SELECT * FROM {} WHERE product = '{}' AND gate_timestamp >= '{}' AND gate_timestamp < '{}'",
            self.tablename, self.product, self.current_timestamp, next_timestamp
        );
        let rows = self.client.query(&query).fetch_all::<Event>().await.ok()?;
        self.current_timestamp = next_timestamp;
        self.buffer = rows.into_iter();
        Some(())
    }
    pub async fn next(&mut self) -> Option<Event> {
        if let Some(row) = self.buffer.next() {
            return Some(row);
        }
        if self.load_data().await.is_none() {
            return None;
        }
        self.buffer.next()
    }
}

pub struct MarketdataPlayer {
    last_update_id: Option<u64>,
    dataprovider: DataProvider,
    orderbook: Orderbook,
    quantity_execution: Decimal,
    model: Model, // создать надструктуру симулятор
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
        let mut last_event: Option<Event> = None;
        let mut best_player_total_price = Decimal::ZERO;
        let mut best_model_price_lower = 0.0;
        let mut best_model_price_upper = 0.0;
        let mut real_total_price = Decimal::ZERO;
        let mut delta_execution = 0;
        let quantity_execution = f64::from_str(&self.quantity_execution.to_string()).unwrap();
        while let Some(event) = self.dataprovider.next().await {
            let cur_event = event.clone();
            match event.event_type.as_str() {
                "depth" => {
                    if let Some(last_event) = last_event {
                        if last_event.event_type == "trade" {
                            best_player_total_price = self
                                .orderbook
                                .best_total_price(self.quantity_execution.clone())
                                .unwrap();
                            best_model_price_lower = (self.model.get_best_price(0.95).0
                                + self.model.last_pbest)
                                * quantity_execution;

                            best_model_price_upper = (self.model.get_best_price(0.95).1
                                + self.model.last_pbest)
                                * quantity_execution;
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
                            println!(
                                "{} {} {} {} {}",
                                best_player_total_price,
                                best_model_price_lower,
                                best_model_price_upper,
                                real_total_price,
                                delta_execution
                            );
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

struct Model {
    last_pbest: f64,
    time_interval: Vec<f64>,
    price_shift: Vec<f64>,
}

impl Model {
    pub fn reinit(pbest: f64) -> Self {
        Self {
            last_pbest: pbest,
            time_interval: Vec::new(),
            price_shift: Vec::new(),
        }
    }
    pub fn update(&mut self, delta_t: i64, price: String) -> Result<()> {
        self.time_interval.push((delta_t as f64).ln());
        let price = self.last_pbest - f64::from_str(&price)?;
        self.price_shift.push(price.abs().sqrt());
        Ok(())
    }
    fn get_best_price(&mut self, p: f64) -> (f64, f64) {
        let num_of_obs = self.time_interval.len() as f64;
        let time_mean = self.time_interval.iter().sum::<f64>() / num_of_obs;
        let price_mean = self.price_shift.iter().sum::<f64>() / num_of_obs;

        let mut variance_time = 0.0;
        let mut variance_price = 0.0;
        let mut covariance_time_price = 0.0;

        for i in 0..num_of_obs as usize {
            variance_time += (self.time_interval[i] - time_mean).powi(2);
            variance_price += (self.price_shift[i] - price_mean).powi(2);
            covariance_time_price +=
                (self.price_shift[i] - price_mean) * (self.time_interval[i] - time_mean);
        }

        let covariance_coef = variance_time.sqrt()
            / (variance_time * variance_price - covariance_time_price.powi(2)).sqrt();
        let f_dist = FisherSnedecor::new(2.0, num_of_obs - 2.0).unwrap();
        let hotelling_stat = (2.0 * (num_of_obs - 1.0) / (num_of_obs * (num_of_obs - 2.0))
            * f_dist.inverse_cdf(p))
        .sqrt();

        let lower_bound = (price_mean - hotelling_stat / covariance_coef).powi(2);
        let upper_bound = (price_mean + hotelling_stat / covariance_coef).powi(2);

        (lower_bound, upper_bound)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let product = "ETHUSDT".to_string();
    let tablename = "marketDataProcessedFull".to_string();
    let start_timestamp = "2024-11-26 10:00:00.000".to_string();
    let quantity_execution = Decimal::from_str("100.100")?;
    let mut marketdata_player =
        MarketdataPlayer::new(product, tablename, start_timestamp, quantity_execution).await;
    marketdata_player.play().await?;
    Ok(())
}
