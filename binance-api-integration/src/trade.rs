use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Trade {
    E: i64,
    s: String,
    t: u64,
    p: String,
    q: String,
}

impl From<&str> for Trade {
    fn from(value: &str) -> Self {
        let trade: Self = serde_json::from_str(value).expect("failed trade deserialize");
        trade
    }
}

impl Trade {
    pub fn event_time(&self) -> i64 { self.E }
    pub fn symbol(&self) -> String { self.s.clone() }
    pub fn trade_id(&self) -> u64 { self.t }
    pub fn price(&self) -> String { self.p.clone() }
    pub fn quantity(&self) -> String { self.q.clone() }
}