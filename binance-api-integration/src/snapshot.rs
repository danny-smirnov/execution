use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(rename = "lastUpdateId")]
    id: u64,
    bids: Vec<(String, String)>,
    asks: Vec<(String, String)>
}

impl From<&str> for Snapshot {
    fn from(value: &str) -> Self {
        let snapshot: Self = serde_json::from_str(value).expect("failed snapshot deserialize");
        snapshot
    }
}

impl Snapshot {
    pub fn iter(&self) -> SnapshotIter {
        SnapshotIter {
            id: self.id,
            bids: self.bids.iter(),
            asks: self.asks.iter(),
        }
    }
}

pub struct SnapshotItem {
    id: u64,
    price: String,
    quantity: String,
    ask_not_bid: bool,
}

impl SnapshotItem {
    pub fn id(&self) -> u64 { self.id }
    pub fn price(&self) -> String { self.price.clone() }
    pub fn quantity(&self) -> String { self.quantity.clone() }
    pub fn ask_not_bid(&self) -> bool { self.ask_not_bid }
}

pub struct SnapshotIter<'a> {
    id: u64,
    bids: std::slice::Iter<'a, (String, String)>,
    asks: std::slice::Iter<'a, (String, String)>,
}

impl<'a> Iterator for SnapshotIter<'a> {
    type Item = SnapshotItem;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((price, quantity)) = self.bids.next() {
            return Some(SnapshotItem {
                id: self.id,
                price: price.clone(),
                quantity: quantity.clone(),
                ask_not_bid: false,
            }); 
        }
        if let Some((price, quantity)) = self.asks.next() {
            return Some(SnapshotItem {
                id: self.id,
                price: price.clone(),
                quantity: quantity.clone(),
                ask_not_bid: true,
            }); 
        }
        None
    }
}