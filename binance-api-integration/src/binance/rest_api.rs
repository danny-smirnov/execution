use crate::event::RawEvent;
use serde_json::Value;

pub struct ExchangeInfo {
    pub weight: u64,
    pub limit: u64,
    // pub symbols_name: Vec<String>,
}

impl ExchangeInfo {
    pub async fn new() -> Self {
        let responce = reqwest::get("https://api.binance.com/api/v3/exchangeInfo")
            .await
            .expect("failed get request \"exchangeInfo\"")
            .text()
            .await
            .expect("failed decode request \"exchangeInfo\"");
        let exchange_info: Value =
            serde_json::from_str(&responce).expect("failed exchange info deserialize");
        let limit = exchange_info["rateLimits"][0]["limit"].as_u64().unwrap();
        // let symbols = exchange_info["symbols"].as_array().unwrap().to_vec();
        // let mut symbols_names = vec![String::new(); symbols.len()];
        // for i in 0..symbols.len() {
        //     symbols_names[i] = symbols[i]["symbol"].as_str().unwrap().to_string();
        // }
        ExchangeInfo {
            weight: 20,
            limit: limit,
            // symbols_name: symbols_names,
        }
    }
}

pub struct SnapshotInfo {
    pub weight: u64,
    pub raw_snapshot: RawEvent,
}

impl SnapshotInfo {
    pub async fn new(symbol: &str) -> Self {
        let symbol = symbol.to_uppercase();
        let url = format!(
            "https://api.binance.com/api/v3/depth?symbol={}&limit=5000",
            symbol,
        );
        let responce = reqwest::get(url)
            .await
            .expect("failed get request \"depth\"")
            .text()
            .await
            .expect("failed decode to text request \"depth\"");
        let raw_snapshot = format!("{}@snapshot{}", symbol, responce);
        SnapshotInfo {
            weight: 250,
            raw_snapshot: RawEvent::RawSnapshot(raw_snapshot)
        }
    }
}