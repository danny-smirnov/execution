use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Depth {
    E: i64,
    s: String,
    U: u64,
    u: u64,
    b: Vec<(String, String)>,
    a: Vec<(String, String)>,
}

impl From<&str> for Depth {
    fn from(value: &str) -> Self {
        let depth: Self = serde_json::from_str(value).expect("failed depth deserialize");
        depth
    }
}

impl Depth {
    pub fn iter(&self) -> DepthIterator {
        DepthIterator {
            E: self.E,
            s: &self.s,
            U: self.U,
            u: self.u,
            asks: self.a.iter(),
            bids: self.b.iter(),
        }
    }   
}

pub struct DepthItem {
    E: i64,
    s: String,
    U: u64,
    u: u64,
    price: String,
    quantity: String,
    ask_not_bid: bool,
}

impl DepthItem {
    pub fn event_time(&self) -> i64 { self.E }
    pub fn symbol(&self) -> String { self.s.clone() }
    pub fn first_update_id(&self) -> u64 { self.U }
    pub fn last_update_id(&self) -> u64 { self.u }
    pub fn price(&self) -> String { self.price.clone() }
    pub fn quantity(&self) -> String { self.quantity.clone() }
    pub fn ask_not_bid(&self) -> bool { self.ask_not_bid } 
}

pub struct DepthIterator<'a> {
    E: i64,
    s: &'a String,
    U: u64,
    u: u64,
    bids: std::slice::Iter<'a, (String, String)>,
    asks: std::slice::Iter<'a, (String, String)>,
}

impl<'a> Iterator for DepthIterator<'a> {
    type Item = DepthItem;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((price, quantity)) = self.bids.next() {
            return Some(DepthItem {
                E: self.E,
                s: self.s.clone(),
                U: self.U,
                u: self.u,
                price: price.clone(),
                quantity: quantity.clone(),
                ask_not_bid: false,
            }); 
        }
        if let Some((price, quantity)) = self.asks.next() {
            return Some(DepthItem {
                E: self.E,
                s: self.s.clone(),
                U: self.U,
                u: self.u,
                price: price.clone(),
                quantity: quantity.clone(),
                ask_not_bid: true,
            });
        }
        None
    }
}