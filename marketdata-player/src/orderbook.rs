use crate::Event;
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
    pub fn update(&mut self, diff: Event) -> Result<()> {
        let price: Price = Decimal::from_str(&diff.price)?;
        let quantity: Quantity = Decimal::from_str(&diff.quantity)?;
        let ask_not_bid = diff.ask_not_bid.unwrap();
        if ask_not_bid {
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
        Ok(())
    }
    pub fn handle_trade(&mut self, trade: Event) -> Result<()> {
        let price: Price = Decimal::from_str(&trade.price)?;
        let quantity: Quantity = Decimal::from_str(&trade.quantity)?;
        if let Some((&key, &value)) = self.asks.iter().next() {
            let remaining_quantity = value - quantity;
            if price >= key {
                if remaining_quantity.eq_zero() {
                    self.asks.remove(&key);
                } else {
                    self.asks.insert(key, remaining_quantity);
                }
            } else {
                if remaining_quantity.eq_zero() {
                    self.bids.remove(&Reverse(key));
                } else {
                    self.bids.insert(Reverse(key), remaining_quantity);
                }
            }
        }
        Ok(())
    }
    pub fn pbest(&mut self) -> f64 {
        let (&price, _) = self.asks.iter().next().unwrap();
        f64::from_str(&price.to_string()).unwrap()
    } 
    pub fn best_total_price(&self, mut amount: Decimal) -> Option<Decimal> {
        let mut total_price = Decimal::ZERO;
        while amount > 0 {
            let (&price, &quantity) = self.asks.iter().next()?;
            total_price += quantity.max(amount) * price;
            amount -= quantity;
        } 
        Some(total_price) 
    }
}
