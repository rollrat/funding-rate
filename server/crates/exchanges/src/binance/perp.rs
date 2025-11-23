use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{BinanceClient, ExchangeError, PerpExchange};
use interface::{Currency, ExchangeId, PerpSnapshot};

const BASE_URL: &str = "https://fapi.binance.com";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinancePremiumIndex {
    symbol: String,
    mark_price: String,
    last_funding_rate: String,
    next_funding_time: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceTicker24h {
    symbol: String,
    quote_volume: String,
    #[serde(default)]
    open_interest: String,
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

            let mark_price: f64 = match p.mark_price.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let funding_rate: f64 = p.last_funding_rate.parse().unwrap_or(0.0);

            let oi_contracts: f64 = t.open_interest.parse().unwrap_or(0.0);
            let oi_usd = oi_contracts * mark_price;

            let vol_24h_usd: f64 = t.quote_volume.parse().unwrap_or(0.0);

            let next_funding_time: Option<DateTime<Utc>> = if p.next_funding_time > 0 {
                DateTime::from_timestamp_millis(p.next_funding_time)
            } else {
                None
            };

            out.push(PerpSnapshot {
                exchange: ExchangeId::Binance,
                symbol: p.symbol,
                currency: Currency::USDT,
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
