mod dataprovider;
mod marketdataplayer;
mod orderbook;

use anyhow::{Ok, Result};
use clickhouse::Row;
use fpdec::Decimal;
use marketdataplayer::MarketdataPlayer;
use serde::Deserialize;
use tokio::{fs::OpenOptions, io::{AsyncBufReadExt, BufReader}};
use std::str::FromStr;

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

#[tokio::main]
async fn main() -> Result<()> {
    let file = OpenOptions::new().read(true).open("symbols.txt").await?;
    let mut lines = BufReader::new(file).lines();
    while let Some(product) = lines.next_line().await? {
        println!("{}", product);
        let tablename = "marketDataSorted".to_string();
        let start_timestamp = "2024-11-26 05:50:00".to_string();
        let quantity_execution = Decimal::from_str("1.01")?;
        println!("Buying {} in amount of {}", product, quantity_execution);
        let mut marketdata_player =
            MarketdataPlayer::new(product, tablename, start_timestamp, quantity_execution).await;
        marketdata_player.play().await?;
    } 
    Ok(())
}