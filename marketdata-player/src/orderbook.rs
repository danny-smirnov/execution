use crate::{Depths, Snapshot};
use anyhow::{Ok, Result};
use fpdec::Decimal;
use std::{cmp::Reverse, collections::BTreeMap, str::FromStr};

pub type Price = Decimal;
pub type Quantity = Decimal;

#[derive(Debug)]
pub struct Orderbook {
    bids: BTreeMap<Reverse<Price>, Quantity>,
    asks: BTreeMap<Price, Quantity>,
}

impl Default for Orderbook {
    fn default() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }
}

impl Orderbook {
    pub fn new(&mut self, snapshot: Snapshot, depths: Depths) -> Result<()> {
        self.bids = BTreeMap::new();
        self.asks = BTreeMap::new();
        for item in snapshot {
            let price: Price = Decimal::from_str(&item.price)?;
            let quantity: Quantity = Decimal::from_str(&item.quantity)?;
            let is_ask = item.ask_not_bid.unwrap();
            if is_ask {
                self.asks.insert(price, quantity);
            } else {
                self.bids.insert(Reverse(price), quantity);
            }
        }
        for item in depths {
            let price: Price = Decimal::from_str(&item.price)?;
            let quantity: Quantity = Decimal::from_str(&item.quantity)?;
            let is_ask = item.ask_not_bid.unwrap();
            if is_ask {
                self.asks.insert(price, quantity);
            } else {
                self.bids.insert(Reverse(price), quantity);
            }
        }
        Ok(())
    }
}
