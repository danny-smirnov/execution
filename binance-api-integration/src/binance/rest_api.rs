use serde_json::Value;

pub async fn exchange_info() -> (u64, u64, Vec<String>) {
    let responce = reqwest::get("https://api.binance.com/api/v3/exchangeInfo")
        .await
        .expect("failed get request \"exchangeInfo\"")
        .text()
        .await
        .expect("failed decode request \"exchangeInfo\"");
    let exchange_info: Value =
        serde_json::from_str(&responce).expect("failed exchange info deserialize");
    let limit = exchange_info["rateLimits"][0]["limit"].as_u64().unwrap();
    let symbols = exchange_info["symbols"].as_array().unwrap().to_vec();
    let mut symbols_names = vec![String::new(); symbols.len()];
    for i in 0..symbols.len() {
        symbols_names[i] = symbols[i]["symbol"].as_str().unwrap().to_string();
    }
    (20, limit, symbols_names)
}

pub async fn snapshot(symbol: String) -> (u64, String) {
    let url = format!(
        "https://api.binance.com/api/v3/depth?symbol={}&limit=5000",
        symbol.to_uppercase(),
    );
    let responce = reqwest::get(url)
        .await
        .expect("failed get request \"depth\"")
        .text()
        .await
        .expect("failed decode request \"depth\"");
    let snapshot = format!("{}@snapshot{}", symbol, responce);
    (250, snapshot)
}
