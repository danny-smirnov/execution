use crate::depth::DepthItem;
use crate::snapshot::SnapshotItem;
use crate::trade::Trade;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
pub struct Event {
    local_unique_id: i64,
    venue_timestamp: i64,
    gate_timestamp: i64,
    event_type: String,
    product: String,
    id1: Option<u64>,
    id2: Option<u64>,
    ask_not_bid: Option<bool>,
    buy_not_sell: Option<bool>,
    price: String,
    quantity: String,
}

impl From<(Trade, i64)> for Event {
    fn from(value: (Trade, i64)) -> Self {
        let (trade, timestamp) = value;
        Event {
            local_unique_id: Event::local_unique_id(),
            venue_timestamp: trade.event_time(),
            gate_timestamp: timestamp,
            event_type: "trade".to_string(),
            product: trade.symbol(),
            id1: Some(trade.trade_id()),
            id2: None,
            ask_not_bid: None,
            buy_not_sell: Some(true),
            price: trade.price(),
            quantity: trade.quantity(),
        }
    }
}

impl From<(DepthItem, i64)> for Event {
    fn from(value: (DepthItem, i64)) -> Self {
        let (depth_item, timestamp) = value;
        Event {
            local_unique_id: Event::local_unique_id(),
            venue_timestamp: depth_item.event_time(),
            gate_timestamp: timestamp,
            event_type: "depth".to_string(),
            product: depth_item.symbol(),
            id1: Some(depth_item.first_update_id()),
            id2: Some(depth_item.last_update_id()),
            ask_not_bid: Some(depth_item.ask_not_bid()),
            buy_not_sell: None,
            price: depth_item.price(),
            quantity: depth_item.quantity(),
        }
    }
}

impl From<(SnapshotItem, &str, i64)> for Event {
    fn from(value: (SnapshotItem, &str, i64)) -> Self {
        let (snapshot_item, symbol, timestamp) = value;
        Event {
            local_unique_id: Event::local_unique_id(),
            venue_timestamp: timestamp,
            gate_timestamp: timestamp,
            event_type: "snapshot".to_string(),
            product: symbol.to_string(),
            id1: Some(snapshot_item.last_update_id()),
            id2: None,
            ask_not_bid: Some(snapshot_item.ask_not_bid()),
            buy_not_sell: None,
            price: snapshot_item.price(),
            quantity: snapshot_item.quantity(),
        }
    }
}

impl Event {
    fn local_unique_id() -> i64 {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        unsafe {
            libc::clock_gettime(libc::CLOCK_MONOTONIC_RAW, &mut ts);
        }
        ts.tv_sec * 1_000_000_000 + ts.tv_nsec
    }
    pub fn to_json(&self) -> String {
        json!({
            "local_unique_id": self.local_unique_id,
            "venue_timestamp": self.venue_timestamp,
            "gate_timestamp": self.gate_timestamp,
            "type": self.event_type,
            "product": self.product,
            "id1": self.id1,
            "id2": self.id2,
            "ask_not_bid": self.ask_not_bid,
            "buy_not_sell": self.buy_not_sell,
            "price": self.price,
            "quantity": self.quantity
        }).to_string()
    }
    pub fn encode(&self) -> Vec<u8> {
        bincode::serialize(self).expect("failed serialize event to bincode")
    }
    pub fn decode(bytes: &[u8]) -> Self {
        let decode: Self =
            bincode::deserialize(bytes).expect("failed deserialize event from bytes");
        decode
    }
}
