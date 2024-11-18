mod binance;
mod depth;
mod event;
mod snapshot;
mod trade;

use binance::{rest_api, websocket_api};
use chrono::Utc;
use depth::Depth;
use event::Event;
use snapshot::Snapshot;
use tokio::{
    io::AsyncWriteExt,
    sync::mpsc,
    time::{sleep, Duration, Instant},
};
use trade::Trade;

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
                    .send(RawEvent::OrderbookSnapshot(snapshot))
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
                .open(format!("{}.bin", Utc::now().format("%d-%m-%Y_%H-%M")))
                .await
                .expect("acceptor: failed to open a file");
            let start_instant = Instant::now();
            while let Some(event) = events_rx.recv().await {
                let timestamp = Utc::now().timestamp_millis();
                match event {
                    RawEvent::Trade(t) => {
                        let data = extract_data(&t).expect("failed extract");
                        let trade = Trade::from(data);
                        let event = Event::from((trade, timestamp));
                        file.write_all(&event.encode_to_binary_form().unwrap())
                            .await
                            .expect("acceptor: failed write \"trade\"");
                    }
                    RawEvent::OrderbookUpdate(obu) => {
                        let data = extract_data(&obu).expect("failed extract");
                        let depth = Depth::from(data);
                        for depth_item in depth.iter() {
                            let event = Event::from((depth_item, timestamp));
                            file.write_all(&event.encode_to_binary_form().unwrap())
                                .await
                                .expect("acceptor: failed write \"depth\"");
                        }
                    }
                    RawEvent::OrderbookSnapshot(obss) => {
                        let (symbol, data) = obss
                            .split_once("@snapshot")
                            .expect("acceptor: failed split \"snapshot\"");
                        let snapshot = Snapshot::from(data);
                        for snapshot_item in snapshot.iter() {
                            let event = Event::from((snapshot_item, symbol, timestamp));
                            file.write_all(&event.encode_to_binary_form().unwrap())
                                .await
                                .expect("acceptor: failed write \"snapshot\"");
                        }
                    }
                }
                if Instant::now() - start_instant >= FILE_TIMER {
                    break;
                }
            }
        }
    });
}

enum RawEvent {
    Trade(String),
    OrderbookUpdate(String),
    OrderbookSnapshot(String),
}
