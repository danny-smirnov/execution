use crate::trade::Trade;
use crate::depth::DepthItem;
use crate::snapshot::SnapshotItem;
use serde::{Deserialize, Serialize};

pub enum RawEvent {
    RawTrade(String),
    RawDepth(String),
    RawSnapshot(String),
}

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

fn local_unique_id() -> i64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC_RAW, &mut ts); }
    ts.tv_sec * 1_000_000_000 + ts.tv_nsec
}

impl Event {
    pub fn from_trade(trade: Trade, timestamp: i64) -> Self {
        Event {
            local_unique_id: local_unique_id(),
            venue_timestamp: trade.event_time(),
            gate_timestamp: timestamp,
            event_type: "trade".to_string(),
            product: trade.symbol(),
            id1: Some(trade.trade_id()),
            id2: None,
            ask_not_bid: None,
            buy_not_sell: Some(trade.market_maker()),
            price: trade.price(),
            quantity: trade.quantity(),
        }
    }

    pub fn from_depth_item(depth_item: DepthItem, timestamp: i64) -> Self {
        Event {
            local_unique_id: local_unique_id(),
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

    pub fn from_snapshot_item(snapshot_item: SnapshotItem, symbol: &str, timestamp: i64) -> Self {
        Event {
            local_unique_id: local_unique_id(),
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

    pub fn as_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(bincode::deserialize(bytes)?)
    }
}
