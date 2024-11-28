use crate::{Diffs, Snapshot};
use anyhow::{Ok, Result};
use fpdec::Decimal;
use std::{cmp::Reverse, collections::BTreeMap, str::FromStr};

pub type Price = Decimal;
pub type Quantity = Decimal;

#[derive(Debug)]
pub struct Orderbook {
    last_update_id: u64,
    bids: BTreeMap<Reverse<Price>, Quantity>,
    asks: BTreeMap<Price, Quantity>,
}

impl Orderbook {
    pub fn from_snapshot(snapshot: Snapshot) -> Result<Self> {
        let mut orderbook = Self {
            last_update_id: snapshot[0].id1.unwrap(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        };
        for iter in snapshot {
            let price: Price = Decimal::from_str(&iter.price)?;
            let quantity: Quantity = Decimal::from_str(&iter.quantity)?;
            let ask_not_bid = iter.ask_not_bid.unwrap();
            if ask_not_bid {
                orderbook.asks.insert(price, quantity);
            } else {
                orderbook.bids.insert(Reverse(price), quantity);
            }
        }
        Ok(orderbook)
    }
    pub fn update(&mut self, diffs: Diffs) -> Result<()> {
        for diff in diffs {
            if diff.id2.unwrap() <= self.last_update_id {
                continue;
            }
            let price: Price = Decimal::from_str(&diff.price)?;
            let quantity: Quantity = Decimal::from_str(&diff.quantity)?;
            let is_ask = diff.ask_not_bid.unwrap();
            if is_ask {
                if quantity.eq_zero() {
                    self.asks.remove(&price);
                } else {
                    self.asks.insert(price, quantity);
                }
            } else {
                if quantity.eq_zero() {
                    self.bids.remove(&Reverse(price));
                } else {
                    self.bids.insert(Reverse(price), quantity);
                }
            }
        }
        Ok(())
    }
}
