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
    let s = String::from("BTCUSDT");
    let timer = 60;
    let limit = 5000;
    let time_await = 60;
    let time_reconnect = 60 * 60 * 12;
    let (tx, rx) = mpsc::channel(100_000);
    tokio::spawn(binance::restapi::snapshot(
        tx.clone(),
        s.clone(),
        timer,
        limit,
    ));
    let url = format!(
        "wss://stream.binance.com:9443/stream?streams={}@trade/{}@depth@100ms",
        s.to_lowercase(),
        s.to_lowercase(),
    );
    tokio::spawn(binance::websocket::create(
        tx.clone(),
        url,
        time_await,
        time_reconnect,
    ));
    tokio::spawn(acceptor(rx, Duration::from_secs(60 * 60)));
    sleep(Duration::from_secs(60)).await;
}

async fn acceptor(mut rx: mpsc::Receiver<String>, time_alive: Duration) -> std::io::Result<()> {
    loop {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(format!("{}.json", Utc::now().format("%d-%m-%Y_%H-%M")))?;
        let start = Instant::now();
        while let Some(msg) = rx.recv().await {
            let timestamp = Utc::now().timestamp_millis();
            if msg.contains("@trade") {
                let data = &msg[33..msg.len() - 1];
                let trade = Trade::from(data);
                let event = Event::from((trade, timestamp));
                write!(file, "{}", event.to_json())?;
            } else if msg.contains("@depth") {
                let data = &msg[39..msg.len() - 1];
                let depth = Depth::from(data);
                for depth_item in depth.iter() {
                    let event = Event::from((depth_item, timestamp));
                    write!(file, "{}", event.to_json())?;
                }
            } else {
                let (symbol, data) = msg.split_once('&').expect("failed split snapshot");
                let snapshot = Snapshot::from(data);
                for snapshot_item in snapshot.iter() {
                    let event = Event::from((snapshot_item, symbol, timestamp));
                    write!(file, "{}", event.to_json())?;
                }
            }
            if Instant::now() - start >= time_alive { break; }
        }
    }
}
