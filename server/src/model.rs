use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExchangeId {
    Binance,
    Bybit,
    Okx,
    Bitget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpSnapshot {
    pub exchange: ExchangeId,
    pub symbol: String,
    pub mark_price: f64,
    pub oi_usd: f64,
    pub vol_24h_usd: f64,
    pub funding_rate: f64, // 0.01 == 1%
    pub next_funding_time: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotSnapshot {
    pub exchange: ExchangeId,
    pub symbol: String,
    pub price: f64,
    pub vol_24h_usd: f64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSnapshot {
    pub exchange: ExchangeId,
    pub symbol: String,
    // 선물 데이터
    pub perp: Option<PerpData>,
    // 현물 데이터
    pub spot: Option<SpotData>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerpData {
    pub mark_price: f64,
    pub oi_usd: f64,
    pub vol_24h_usd: f64,
    pub funding_rate: f64, // 0.01 == 1%
    pub next_funding_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotData {
    pub price: f64,
    pub vol_24h_usd: f64,
}
