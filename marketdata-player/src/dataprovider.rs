use chrono::{Duration, NaiveDateTime};
use clickhouse::{query, Client};
use std::vec::IntoIter;

use crate::Event;

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
    async fn load_marketdata(&mut self) -> Option<()> {
        let next_timestamp = self.current_timestamp + Duration::minutes(5);
        let query = format!("SELECT local_unique_id, venue_timestamp, gate_timestamp, event_type, product, id1, id2, ask_not_bid, buy_not_sell, price, quantity FROM {} WHERE product = '{}' AND gate_timestamp >= toDateTime64('{}', 3, 'UTC') AND gate_timestamp < toDateTime64('{}', 3, 'UTC')",
        self.tablename, self.product, self.current_timestamp, next_timestamp);
        self.current_timestamp = next_timestamp;
        let events = self.client.query(&query).fetch_all::<Event>().await.ok();
        if let Some(events) = events {
            self.buffer = events.into_iter();
            return Some(());
        }
        None
    }
    pub async fn next(&mut self) -> Option<Event> {
        if let Some(event) = self.buffer.next() {
            return Some(event);
        }
        if let Some(_) = self.load_marketdata().await {
            return self.buffer.next();
        }
        None
    }
    pub fn product(&self) -> String {
        self.product.clone()
    }
}
