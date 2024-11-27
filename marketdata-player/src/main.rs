mod orderbook;

use anyhow::{Ok, Result};
use clickhouse::{Client, Row};
use orderbook::Orderbook;
use serde::Deserialize;

pub type Depths = Vec<Event>;
pub type Snapshot = Vec<Event>;

#[derive(Row, Deserialize, Debug)]
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
    let client = Client::default()
        .with_url("http://127.0.1.1:8123")
        .with_user("default")
        .with_database("default")
        .with_compression(clickhouse::Compression::None);
    let product = "\'ETHUSDT\'";
    let table = "marketDataProcessedFull";
    let timestamp = "\'2024-11-26 10:00:00.000\'";
    let mut orderbook = Orderbook::default();
    let snapshot_query = format!("WITH 
    (
         SELECT 
             *, 
             CAST(toDateTime64({timestamp}, 3, 'UTC'), 'Float64') - CAST(venue_timestamp, 'Float64') AS t_diff
         FROM {table}
         WHERE 
             product = {product} 
             AND event_type = 'snapshot'
             AND venue_timestamp < toDateTime64({timestamp}, 3, 'UTC')
         ORDER BY 
             t_diff ASC
         LIMIT 1
     ) AS needed_snapshot
 SELECT count(*) 
 FROM 
     marketDataProcessedFull
 WHERE 
     id1 = needed_snapshot.id1");
    let depths_query = format!("WITH (
        SELECT
        *,
        CAST(toDateTime64({timestamp}, 3, 'UTC'), 'Float64') - CAST(venue_timestamp, 'Float64') AS t_diff
        FROM marketDataProcessedFull
        WHERE (product = {product}) AND (event_type = 'snapshot') AND (venue_timestamp < toDateTime64({timestamp}, 3, 'UTC'))
        ORDER BY t_diff ASC
        LIMIT 1
        ) AS needed_snapshot
        SELECT *
        FROM marketDataProcessedFull
        WHERE (venue_timestamp > needed_snapshot.venue_timestamp) AND (venue_timestamp <= toDateTime64({timestamp}, 3, 'UTC')) AND (event_type = 'depth')");
    let snapshot: Snapshot = client.query(&snapshot_query).fetch_all::<Event>().await?;
    let depths: Depths = client.query(&depths_query).fetch_all::<Event>().await?;
    orderbook.new(snapshot, depths)?;
    println!("{:?}", orderbook);

    Ok(())
}
