use std::{cmp::Reverse, collections::BTreeMap};

use fpdec::Decimal;

fn main() {
    let data_provider = DataProvider::new();
    let mut simulator = Simulator::default();
    let market_data = data_provider.get_market_data_stream();
    for md in market_data {
        simulator.insert(md);
    }
}

pub struct Stream;

impl IntoIterator for Stream {
    type Item = MarketData;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}

pub struct DataProvider;

impl DataProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn get_market_data_stream(&self) -> Stream {
        Stream
    }
}
/*
[A]
-
Spread
-
[B]
*/

#[derive(Default)]
struct Orderbook {
    asks: BTreeMap<Decimal, Decimal>,
    bids: BTreeMap<Reverse<Decimal>, Decimal>,
    ask_depth: Decimal,
    bid_depth: Decimal,
}

impl Orderbook {
    pub fn update_ask(&mut self, pl: PriceLevel) {
        if pl.quantity == Decimal::ZERO {
            self.asks.remove(&pl.price);
            return;
        }
        self.asks.insert(pl.price, pl.quantity);
        self.ask_depth = self
            .asks
            .iter()
            .fold(Decimal::ZERO, |acc, (price, quantity)| {
                acc + price * quantity
            });
    }

    pub fn update_bid(&mut self, pl: PriceLevel) {
        if pl.quantity == Decimal::ZERO {
            self.bids.remove(&Reverse(pl.price));
            return;
        }
        self.bids.insert(Reverse(pl.price), pl.quantity);
        self.bid_depth = self
            .bids
            .iter()
            .fold(Decimal::ZERO, |acc, (price, quantity)| {
                acc + price.0 * quantity
            });
    }

    pub fn depths(&self) -> (Decimal, Decimal) {
        (self.ask_depth, self.bid_depth)
    }

    pub fn spread(&self) -> Option<Decimal> {
        let Some((&ap, _)) = self.asks.first_key_value() else {
            return None;
        };
        let Some((&bp, _)) = self.bids.first_key_value() else {
            return None;
        };
        Some(ap - bp.0)
    }
}

#[derive(Default)]
pub struct Simulator {
    orderbook: Orderbook,
}

impl Simulator {
    pub fn insert(&mut self, market_data: MarketData) {
        match market_data {
            MarketData::OrderbookSnapshot(obss) => {
                // for pl in obss.asks {
                //     self.orderbook.update_ask(pl);
                // }
                // for pl in obss.bids {
                //     self.orderbook.update_bid(pl);
                // }
                // pohui
            }
            MarketData::OrderbookUpdate(obu) => {
                for pl in obu.asks {
                    self.orderbook.update_ask(pl);
                }
                for pl in obu.bids {
                    self.orderbook.update_bid(pl);
                }
            }
            MarketData::Trade(_) => {
                // TODO
            }
        }
    }
}

pub enum MarketData {
    Trade(Trade),
    OrderbookSnapshot(OrderbookSnapshot),
    OrderbookUpdate(OrderbookUpdate),
}

pub enum TradeKind {
    Buy,
    Sell,
}

pub struct PriceLevel {
    price: Decimal,
    quantity: Decimal,
}

pub struct Trade {
    timestamp: i64,
    price_level: PriceLevel,
    kind: Option<TradeKind>,
}

pub struct OrderbookSnapshot {
    timestamp: i64,
    bids: Vec<PriceLevel>,
    asks: Vec<PriceLevel>,
}

pub struct OrderbookUpdate {
    timestamp: i64,
    bids: Vec<PriceLevel>,
    asks: Vec<PriceLevel>,
}
