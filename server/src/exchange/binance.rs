use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Deserialize;

use crate::model::{ExchangeId, PerpSnapshot};

use super::{ExchangeError, PerpExchange};

const BASE_URL: &str = "https://fapi.binance.com";

#[derive(Clone)]
pub struct BinanceClient {
    http: reqwest::Client,
}

impl BinanceClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct BinancePremiumIndex {
    symbol: String,
    markPrice: String,
    lastFundingRate: String,
    nextFundingTime: i64,
}

#[derive(Debug, Deserialize)]
struct BinanceTicker24h {
    symbol: String,
    quoteVolume: String,
    #[serde(default)]
    openInterest: String,
}

#[async_trait]
impl PerpExchange for BinanceClient {
    fn id(&self) -> ExchangeId {
        ExchangeId::Binance
    }

    async fn fetch_all(&self) -> Result<Vec<PerpSnapshot>, ExchangeError> {
        // 1) funding / mark price info
        let premium: Vec<BinancePremiumIndex> = self
            .http
            .get(format!("{BASE_URL}/fapi/v1/premiumIndex"))
            .send()
            .await?
            .json()
            .await?;

        // 2) 24h ticker
        let tickers: Vec<BinanceTicker24h> = self
            .http
            .get(format!("{BASE_URL}/fapi/v1/ticker/24hr"))
            .send()
            .await?
            .json()
            .await?;

        let mut ticker_map: HashMap<String, BinanceTicker24h> = HashMap::new();
        for t in tickers {
            ticker_map.insert(t.symbol.clone(), t);
        }

        let now = Utc::now();

        let mut out = Vec::new();

        for p in premium {
            if !p.symbol.ends_with("USDT") {
                continue; // 선형 USDT perp만
            }

            let t = match ticker_map.get(&p.symbol) {
                Some(t) => t,
                None => continue,
            };

            let mark_price: f64 = match p.markPrice.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let funding_rate: f64 = p.lastFundingRate.parse().unwrap_or(0.0);

            let oi_contracts: f64 = t.openInterest.parse().unwrap_or(0.0);
            let oi_usd = oi_contracts * mark_price;

            let vol_24h_usd: f64 = t.quoteVolume.parse().unwrap_or(0.0);

            let next_funding_time: Option<DateTime<Utc>> = if p.nextFundingTime > 0 {
                NaiveDateTime::from_timestamp_millis(p.nextFundingTime)
                    .map(|naive| DateTime::<Utc>::from_utc(naive, Utc))
            } else {
                None
            };

            out.push(PerpSnapshot {
                exchange: ExchangeId::Binance,
                symbol: p.symbol,
                mark_price,
                oi_usd,
                vol_24h_usd,
                funding_rate,
                next_funding_time,
                updated_at: now,
            });
        }

        Ok(out)
    }
}
