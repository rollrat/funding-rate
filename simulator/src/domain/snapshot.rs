#[derive(Debug, Clone)]
pub struct MarketSnapshot {
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub last_trade_price: Option<f64>,
}

