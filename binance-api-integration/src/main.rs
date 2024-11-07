mod binance;
mod depth;
mod event;
mod snapshot;
mod trade;

use chrono::Utc;
use depth::Depth;
use event::Event;
use snapshot::Snapshot;
use std::io::Write;
use tokio::{
    sync::mpsc,
    time::{sleep, Duration, Instant},
};
use trade::Trade;

#[tokio::main]
async fn main() {
    let main_timer = 60;
    let snapshot_timer = 60;
    let snapshot_limit = 5000;
    let acceptor_timer = 3600;
    let ws_time_await = 60;
    let ws_time_reconnect = 60 * 60 * 12;
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let s = "BTCUSDT";
    tokio::spawn(binance::restapi::snapshot(
            tx.clone(),
            s.to_string(),
            snapshot_timer,
            snapshot_limit,
    ));
    let url = format!(
        "wss://stream.binance.com:9443/stream?streams={}@trade/{}@depth@100ms",
        s.to_lowercase(),
        s.to_lowercase(),
    );
    tokio::spawn(binance::websocket::create(
        tx.clone(),
        url,
        ws_time_await,
        ws_time_reconnect,
    ));
    tokio::spawn(acceptor(rx, Duration::from_secs(acceptor_timer)));
    sleep(Duration::from_secs(main_timer)).await;
}

async fn acceptor(mut rx: mpsc::UnboundedReceiver<String>, timer: Duration) -> std::io::Result<()> {
    loop {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(format!("{}.bin", Utc::now().format("%d-%m-%Y_%H-%M")))?;
        let start = Instant::now();
        while let Some(msg) = rx.recv().await {
            let timestamp = Utc::now().timestamp_millis();
            if msg.contains("@trade") {
                let data = &msg[33..msg.len() - 1];
                let trade = Trade::from(data);
                let event = Event::from((trade, timestamp));
                file.write_all(&event.encode())?;
            } else if msg.contains("@depth") {
                let data = &msg[39..msg.len() - 1];
                let depth = Depth::from(data);
                for depth_item in depth.iter() {
                    let event = Event::from((depth_item, timestamp));
                    file.write_all(&event.encode())?;
                }
            } else {
                let (symbol, data) = msg.split_once('&').expect("failed split snapshot");
                let snapshot = Snapshot::from(data);
                for snapshot_item in snapshot.iter() {
                    let event = Event::from((snapshot_item, symbol, timestamp));
                    file.write_all(&event.encode())?;
                }
            }
            if Instant::now() - start >= timer { break; }
        }
    }
}
