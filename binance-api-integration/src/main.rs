mod event;
mod trade;
mod depth;
mod binance;
mod snapshot;

use chrono::Utc;
use trade::Trade;
use depth::Depth;
use tokio::{
    sync::mpsc,
    io::AsyncWriteExt,
    time::{sleep, Duration, Instant},
};
use snapshot::Snapshot;
use event::{Event, RawEvent};
use binance::{rest_api, websocket_api};

const TIMER: Duration = Duration::from_secs(60);
const FILE_TIMER: Duration = Duration::from_secs(3600);
const SNAPSHOT_TIMER: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() {
    let (events_tx, events_rx) = mpsc::unbounded_channel::<RawEvent>();
    acceptor(events_rx).await;
    let (mut weight, limit, symbols) = rest_api::exchange_info().await;
    let mut counter = 0;
    let mut url = String::from("wss://stream.binance.com:9443/stream?streams=");
    for symbol in symbols.iter() {
        let symbol = symbol.to_lowercase();
        url.push_str(&format!("{}@trade/{}@depth@100ms/", symbol, symbol));
        counter += 1;
        if counter == 300 {
            url.pop();
            websocket_api::create(events_tx.clone(), url).await;
            url = String::from("wss://stream.binance.com:9443/stream?streams=");
            counter = 0;
        }
    }
    if counter > 0 {
        url.pop();
        websocket_api::create(events_tx.clone(), url).await;
    }
    tokio::spawn(async move {
        loop {
            for symbol in symbols.iter() {
                let (s_weight, snapshot) = rest_api::snapshot(symbol.to_string()).await;
                events_tx
                    .send(RawEvent::RawSnapshot(snapshot))
                    .expect("failed send to channel");
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

async fn acceptor(mut events_rx: mpsc::UnboundedReceiver<RawEvent>) {
    fn extract_data(msg: &str) -> Option<&str> {
        let key = "\"data\":{";
        let start_idx = msg.find(key)? + key.len() - 1;
        Some(&msg[start_idx..msg.len() - 1])
    }
    
    tokio::task::spawn(async move {
        loop {
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(format!("{}.bin", Utc::now().format("%d-%m-%Y %H-%M-%S")))
                .await
                .expect("acceptor: failed to open a file");
            let start_instant = Instant::now();
            while let Some(event) = events_rx.recv().await {
                let timestamp = Utc::now().timestamp_millis();
                match event {
                    RawEvent::RawTrade(raw_trade) => {
                        let data = extract_data(&raw_trade).expect("failed extract \"raw trade\"");
                        let trade = Trade::from(data);
                        let event = Event::from_trade(trade, timestamp);
                        file.write_all(&event.as_bytes().unwrap())
                            .await
                            .expect("acceptor: failed write \"trade\"");
                    }
                    RawEvent::RawDepth(raw_depth) => {
                        let data = extract_data(&raw_depth).expect("failed extract \"raw depth\"");
                        let depth = Depth::from(data);
                        for depth_item in depth.iter() {
                            let event = Event::from_depth_item(depth_item, timestamp);
                            file.write_all(&event.as_bytes().unwrap())
                                .await
                                .expect("acceptor: failed write \"depth\"");
                        }
                    }
                    RawEvent::RawSnapshot(raw_snapshot) => {
                        let (symbol, data) = raw_snapshot
                            .split_once("@snapshot")
                            .expect("failed split \"raw snapshot\"");
                        let snapshot = Snapshot::from(data);
                        for snapshot_item in snapshot.iter() {
                            let event = Event::from_snapshot_item(snapshot_item, symbol, timestamp);
                            file.write_all(&event.as_bytes().unwrap())
                                .await
                                .expect("acceptor: failed write \"snapshot\"");
                        }
                    }
                }
                if Instant::now() - start_instant >= FILE_TIMER { break; }
            }
        }
    });
}
