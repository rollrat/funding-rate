use chrono::{DateTime, Utc};
use serde::Serialize;
use crate::domain::order::OrderSide;

#[derive(Debug, Clone, Serialize)]
pub struct Trade {
    pub price: f64,
    pub quantity: f64,
    pub side: OrderSide, // 매수 주문인지 매도 주문인지
    pub timestamp: DateTime<Utc>,
}

