mod binance;
mod depth;
mod event;
mod snapshot;
mod trade;

use argh::FromArgs;
use binance::{rest_api, websocket_api};
use depth::Depth;
use event::{Event, RawEvent};
use humantime::parse_duration;
use snapshot::Snapshot;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
    time::{sleep, Duration, Instant},
};
use trade::Trade;

#[derive(FromArgs)]
/// Binance api integration
struct Options {
    /// duration for data collection, e.g., "1day 7hours 43min"
    #[argh(option)]
    runtime: String,
    /// path to the file containing symbols
    #[argh(option)]
    symbols_path: String,
    /// path to save the market data
    #[argh(option)]
    output_path: Option<String>,
}

#[tokio::main]
async fn main() {
    let options: Options = argh::from_env();
    let timer = parse_duration(&options.runtime)
        .expect("failed to parse duration")
        .as_secs();
    let mut file = File::open(options.symbols_path)
        .await
        .expect("failed open products file");
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .await
        .expect("failed file read to string");
    let symbols: Vec<String> = buffer
        .split('\n')
        .map(|s| s.trim().to_lowercase())
        .collect();
    let exchange_info = rest_api::ExchangeInfo::new().await;
    let (events_tx, events_rx) = mpsc::unbounded_channel::<RawEvent>();
    acceptor(events_rx, options.output_path).await;
    let base_endpoint = String::from("wss://stream.binance.com:9443/stream?streams=");
    let mut ws_urls: Vec<String> = vec![base_endpoint.clone()];
    let mut counter = 0;
    for s in symbols.iter() {
        let streams = format!("{s}@trade/{s}@depth@100ms/");
        if let Some(last) = ws_urls.last_mut() {
            last.push_str(&streams);
            counter += 1;
            if counter == 300 {
                ws_urls.push(base_endpoint.clone());
            }
        }
    }
    for mut url in ws_urls {
        url.pop();
        websocket_api::open_stream(events_tx.clone(), url).await;
    }
    tokio::spawn(async move {
        let mut weight = exchange_info.weight;
        loop {
            let mut time_await: i64 = 3600;
            for s in symbols.iter() {
                let snapshot_info = rest_api::SnapshotInfo::new(s).await;
                events_tx
                    .send(snapshot_info.raw_snapshot)
                    .expect("failed to send to channel");
                weight += snapshot_info.weight;
                if weight >= exchange_info.limit - snapshot_info.weight {
                    sleep(Duration::from_secs(60)).await;
                    time_await -= 60;
                    weight = 0;
                }
            }
            if time_await > 0 {
                sleep(Duration::from_secs(time_await as u64)).await;
            }
        }
    });
    sleep(Duration::from_secs(timer)).await;
}

async fn acceptor(mut events_rx: mpsc::UnboundedReceiver<RawEvent>, output_path: Option<String>) {
    fn extract_data(msg: &str) -> Option<&str> {
        let key = "\"data\":{";
        let start_idx = msg.find(key)? + key.len() - 1;
        Some(&msg[start_idx..msg.len() - 1])
    }
    let path = match output_path {
        Some(mut output_path) => {
            output_path.push_str("/marketdata");
            output_path
        }
        None => "marketdata".to_string(),
    };
    tokio::fs::create_dir_all(path.clone())
        .await
        .expect("failed create a directory");
    let file_timer = Duration::from_secs(3600);
    tokio::task::spawn(async move {
        loop {
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(format!(
                    "{}/{}.bin",
                    path,
                    chrono::Utc::now().format("%d-%m-%Y %H-%M-%S")
                ))
                .await
                .expect("acceptor: failed to open a file");
            let start_instant = Instant::now();
            while let Some(event) = events_rx.recv().await {
                let timestamp = chrono::Utc::now().timestamp_millis();
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
                if Instant::now() - start_instant >= file_timer { break; }
            }
        }
    });
}
