use clickhouse::Row;
use serde::Deserialize;
use clickhouse::{sql::Identifier, Client};

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
async fn main() -> anyhow::Result<()> {
    let client = Client::default()
        .with_url("http://localhost:8123")
        .with_user("default")
        .with_database("default")
        .with_compression(clickhouse::Compression::None);
    let table = "marketDataProcessedV";
    let mut cursor = client
        .query("SELECT * FROM ?")
        .bind(Identifier(table))
        .fetch::<Event>()?;
    while let Some(event) = cursor.next().await? {        
        println!("{:?}", event);
    }
    Ok(())
}
