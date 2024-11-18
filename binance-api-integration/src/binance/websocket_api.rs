use futures_util::{SinkExt, StreamExt};
use tokio::{
    sync::mpsc,
    time::{sleep, Duration, Instant},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::RawEvent;

pub const TIME_AWAIT: Duration = Duration::from_secs(60);
pub const TIME_RECONNECT: Duration = Duration::from_secs(43200);

pub async fn create(sender: mpsc::UnboundedSender<RawEvent>, url: String) {
    tokio::spawn(async move {
        loop {
            connect(sender.clone(), url.clone())
                .await
                .expect("failed to establish websocket connection");
            sleep(TIME_RECONNECT).await;
        }
    });
}

pub async fn connect(
    events_tx: mpsc::UnboundedSender<RawEvent>,
    url: String,
) -> anyhow::Result<()> {
    let (ws_stream, _) = connect_async(url).await?;
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
                    if text.contains("@trade") {
                        events_tx
                            .send(RawEvent::Trade(text.clone()))
                            .expect("failed to send to events channel");
                    }
                    if text.contains("@depth") {
                        events_tx
                            .send(RawEvent::OrderbookUpdate(text))
                            .expect("failed to send to events channel");
                    }
                }
                _ => {}
            }
            if Instant::now() - start_instant >= lifetime {
                break;
            }
        }
    });
    Ok(())
}
