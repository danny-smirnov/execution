pub mod restapi {
    use tokio::{
        sync::mpsc,
        time::{sleep, Duration},
    };
    /// The `snapshot` function periodically fetches order book data for a specified symbol
    /// from Binance's REST API and sends it through a channel for further processing.
    ///
    /// # Arguments
    /// - `tx` - `mpsc::Sender<String>`: A channel for sending the fetched data, allowing other parts
    ///   of the program to process it.
    /// - `symbol` - `String`: The symbol for the trading pair on Binance (e.g., "BTCUSDT").
    /// - `timer` - `u64`: The time interval (in seconds) between API requests.
    /// - `limit` - `u64`: Specifies the maximum number of entries in the API response (order book depth).
    pub async fn snapshot(tx: mpsc::Sender<String>, symbol: String, timer: u64, limit: u64) {
        loop {
            let url = format!(
                "https://api.binance.com/api/v3/depth?symbol={}&limit={}",
                symbol.to_uppercase(),
                limit
            );
            let responce = reqwest::get(url).await.unwrap().text().await.unwrap();
            let snapshot = format!("{}&{}", symbol, responce);
            tx.send(snapshot).await.unwrap();
            sleep(Duration::from_secs(timer)).await;
        }
    }
}

pub mod websocket {
    use futures_util::{SinkExt, StreamExt};
    use tokio::{
        sync::mpsc,
        time::{sleep, Duration, Instant},
    };
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    /// Create auto-reconnect websocket connection.
    ///
    /// # Parameters
    /// - `tx`: `mpsc::Sender<String>` — Channel for sending messages received from the server.
    /// - `url`: `String` — The URL for connecting to the WebSocket.
    /// - `time_await`: `u64` — Delay (in seconds) before reconnecting in case of disconnection.
    /// - `time_reconnect`: `u64` — Duration (in seconds) that the connection should remain active.
    pub async fn create(
        tx: mpsc::Sender<String>,
        url: String,
        time_await: u64,
        time_reconnect: u64,
    ) {
        loop {
            open(
                tx.clone(),
                url.clone(),
                Duration::from_secs(time_reconnect + time_await),
            )
            .await;
            sleep(Duration::from_secs(time_reconnect)).await;
        }
    }
    /// Open a websocket connection and handles incoming messages from the server, including responding to `ping`
    /// and receiving text data.
    ///
    /// # Parameters
    /// - `tx`: `mpsc::Sender<String>` — Channel to forward text messages from the server to other parts of the program.
    /// - `url`: `String` — URL for connecting to the WebSocket.
    /// - `time_alive`: `Duration` — Maximum duration the connection should remain active, after which it closes.
    async fn open(tx: mpsc::Sender<String>, url: String, time_alive: Duration) {
        let (ws_stream, _) = connect_async(url).await.expect("failed connect");
        let (mut ws_tx, mut ws_rx) = ws_stream.split();
        let start = Instant::now();
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
                        tx.send(text).await.expect("failed send to channel");
                    }
                    _ => {}
                }
                if Instant::now() - start >= time_alive {
                    break;
                }
            }
        });
    }
}
