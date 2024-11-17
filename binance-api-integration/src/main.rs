mod binance;
mod depth;
mod event;
mod snapshot;
mod trade;

use binance::{restapi, websocket};
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

const TIMER: Duration = Duration::from_secs(60);
const FILE_TIMER: Duration = Duration::from_secs(3600);
const SNAPSHOT_TIMER: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() {
    let sender = acceptor().await;
    let (mut weight, limit, symbols) = restapi::exchange_info().await;
    let c_symbols = symbols.clone();
    let mut counter = 0;
    let mut url = String::from("wss://stream.binance.com:9443/stream?streams=");
    for symbol in symbols {
        let symbol = symbol.to_lowercase();
        url.push_str(&format!("{}@trade/{}@depth@100ms/", symbol, symbol));
        counter += 1;
        if counter == 300 {
            url.pop();
            websocket::create(sender.clone(), url).await;
            url = String::from("wss://stream.binance.com:9443/stream?streams=");
            counter = 0;
        }
    }
    if counter > 0 {
        url.pop();
        websocket::create(sender.clone(), url).await;
    }
    tokio::spawn(async move {
        loop {
            for symbol in c_symbols.iter() {
                let (s_weight, snapshot) = restapi::snapshot(symbol.to_string()).await;
                sender.send(snapshot).expect("failed send to channel");
                weight += s_weight;
                if weight >= limit - s_weight {
                    sleep(SNAPSHOT_TIMER).await;
                    weight = 0;
                }
            }
        }
    });
    sleep(TIMER).await;
}

fn extract_data(msg: &str) -> Option<&str> {
    let key = "\"data\":{";
    let start_idx = msg.find(key)? + key.len() - 1;
    Some(&msg[start_idx..msg.len() - 1])
}

async fn acceptor() -> mpsc::UnboundedSender<String> {
    let (sender, mut receiver) = mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        loop {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(format!("{}.bin", Utc::now().format("%d-%m-%Y_%H-%M")))
                .expect("acceptor: failed open");
            let start_instant = Instant::now();
            while let Some(msg) = receiver.recv().await {
                let timestamp = Utc::now().timestamp_millis();
                if msg.contains("@trade") {
                    let data = extract_data(&msg).expect("failed extract");
                    let trade = Trade::from(data);
                    let event = Event::from((trade, timestamp));
                    file.write_all(&event.encode())
                        .expect("acceptor: failed write \"trade\"");
                } else if msg.contains("@depth") {
                    let data = extract_data(&msg).expect("failed extract");
                    let depth = Depth::from(data);
                    for depth_item in depth.iter() {
                        let event = Event::from((depth_item, timestamp));
                        file.write_all(&event.encode())
                            .expect("acceptor: failed write \"depth\"");
                    }
                } else if msg.contains("@snapshot") {
                    let (symbol, data) = msg
                        .split_once("@snapshot")
                        .expect("acceptor: failed split \"snapshot\"");
                    println!("{}", symbol);
                    let snapshot = Snapshot::from(data);
                    for snapshot_item in snapshot.iter() {
                        let event = Event::from((snapshot_item, symbol, timestamp));
                        file.write_all(&event.encode())
                            .expect("acceptor: failed write \"snapshot\"");
                    }
                }
                if Instant::now() - start_instant >= FILE_TIMER { break; }
            }
        }
    });
    sender
}
