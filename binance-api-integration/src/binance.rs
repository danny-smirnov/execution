pub mod restapi {
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
}

pub mod websocket {
    use futures_util::{SinkExt, StreamExt};
    use tokio::{
        sync::mpsc,
        time::{sleep, Duration, Instant},
    };
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    pub const TIME_AWAIT: Duration = Duration::from_secs(60);
    pub const TIME_RECONNECT: Duration = Duration::from_secs(43200);

    pub async fn create(sender: mpsc::UnboundedSender<String>, url: String) {
        tokio::spawn(async move {
            loop {
                connect(sender.clone(), url.clone()).await;
                sleep(TIME_RECONNECT).await;
            }
        });
    }

    pub async fn connect(sender: mpsc::UnboundedSender<String>, url: String) {
        let (ws_stream, _) = connect_async(url).await.expect("failed connect");
        let (mut ws_tx, mut ws_rx) = ws_stream.split();
        let start_instant = Instant::now();
        let lifetime = TIME_AWAIT + TIME_RECONNECT;
        tokio::spawn(async move {
            while let Some(msg) = ws_rx.next().await {
                match msg {
                    Ok(Message::Ping(ping_data)) => {
                        ws_tx
                            .send(Message::Pong(ping_data))
                            .await
                            .expect("failed send pong");
                    }
                    Ok(Message::Text(text)) => {
                        sender.send(text.clone()).expect("failed send to channel");
                    }
                    _ => {}
                }
                if Instant::now() - start_instant >= lifetime {
                    break;
                }
            }
        });
    }
}
