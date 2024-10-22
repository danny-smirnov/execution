use std::collections::BTreeMap;

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
    bids: BTreeMap<Decimal, Decimal>,
}

impl Orderbook {
    pub fn update_ask(&mut self, pl: PriceLevel) {
        if pl.quantity == Decimal::ZERO {
            self.asks.remove(&pl.price);
            return;
        }
        self.asks.insert(pl.price, pl.quantity);
    }

    pub fn update_bid(&mut self, pl: PriceLevel) {
        if pl.quantity == Decimal::ZERO {
            self.bids.remove(&pl.price);
            return;
        }
        self.bids.insert(pl.price, pl.quantity);
    }
}

#[derive(Default)]
pub struct Simulator {
    orderbook: Orderbook,
}

impl Simulator {
    pub fn insert(&mut self, market_data: MarketData) {
        match market_data {
            MarketData::OrderbookUpdate(obu) => {
                for pl in obu.asks {
                    self.orderbook.update_ask(pl);
                }
                for pl in obu.bids {
                    self.orderbook.update_bid(pl);
                }
            }
            MarketData::Trade(_) => {}
        }
    }
}

pub enum MarketData {
    Trade(Trade),
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

pub struct OrderbookUpdate {
    timestamp: i64,
    bids: Vec<PriceLevel>,
    asks: Vec<PriceLevel>,
}
