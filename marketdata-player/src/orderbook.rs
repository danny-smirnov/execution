use crate::diff::Diff;
use crate::snapshot::Snapshot;

use fpdec::Decimal;

use std::collections::BTreeMap;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug)]
pub struct Orderbook {
    bids: BTreeMap<Decimal, Decimal>,
    asks: BTreeMap<Decimal, Decimal>,
}

impl From<Snapshot> for Orderbook {
    fn from(snapshot: Snapshot) -> Self {
        let mut orderbook = Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        };
        for bid in snapshot.bids().iter() {
            let price = Decimal::from_str(&bid.0).unwrap();
            let quantity = Decimal::from_str(&bid.1).unwrap();
            orderbook.bids.insert(price, quantity);
        }
        for ask in snapshot.asks().iter() {
            let price = Decimal::from_str(&ask.0).unwrap();
            let quantity = Decimal::from_str(&ask.1).unwrap();
            orderbook.asks.insert(price, quantity);
        }
        orderbook
    }
}

impl Orderbook {
    pub fn update(&mut self, diff: Diff) {
        for bid in diff.bids().iter() {
            let price = Decimal::from_str(&bid.0).unwrap();
            let quantity = Decimal::from_str(&bid.1).unwrap();
            if quantity.eq_zero() {
                self.bids.remove(&price);
            } else {
                self.bids.insert(price, quantity);
            }
        }
        for ask in diff.asks().iter() {
            let price = Decimal::from_str(&ask.0).unwrap();
            let quantity = Decimal::from_str(&ask.1).unwrap();
            if quantity.eq_zero() {
                self.asks.remove(&price);
            } else {
                self.asks.insert(price, quantity);
            }
        }
    }
}

impl Display for Orderbook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        std::process::Command::new("clear").status().unwrap();
        let mut output = String::new();
        output += "bids\n";
        for (price, quantity) in self.bids.iter() {
            output += &price.to_string();
            output += " ";
            output += &quantity.to_string();
            output += "\n";
        }
        output += "asks\n";
        for (price, quantity) in self.asks.iter() {
            output += &price.to_string();
            output += " ";
            output += &quantity.to_string();
            output += "\n";
        }
        write!(f, "{}", output)
    }
}