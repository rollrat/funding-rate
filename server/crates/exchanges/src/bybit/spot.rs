use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use crate::{BybitClient, ExchangeError, SpotExchange};
use interface::{Currency, ExchangeId, SpotSnapshot};

const BASE_URL: &str = "https://api.bybit.com";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BybitSpotTicker {
    symbol: String,
    #[serde(default)]
    last_price: String,
    #[serde(default)]
    turnover24h: String,
}

#[derive(Debug, Deserialize)]
struct BybitSpotTickerResult {
    #[allow(dead_code)]
    category: String,
    list: Vec<BybitSpotTicker>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BybitSpotTickerResponse {
    ret_code: i32,
    ret_msg: String,
    result: BybitSpotTickerResult,
}

#[async_trait]
impl SpotExchange for BybitClient {
    fn id(&self) -> ExchangeId {
        ExchangeId::Bybit
    }

    async fn fetch_all(&self) -> Result<Vec<SpotSnapshot>, ExchangeError> {
        let url = format!("{BASE_URL}/v5/market/tickers?category=spot");
        let response: BybitSpotTickerResponse = self.http.get(&url).send().await?.json().await?;

        if response.ret_code != 0 {
            return Err(ExchangeError::Other(format!(
                "Bybit API error (spot): {} - {}",
                response.ret_code, response.ret_msg
            )));
        }

        let now = Utc::now();
        let mut out = Vec::new();

        for ticker in response.result.list {
            if !ticker.symbol.ends_with("USDT") {
                continue; // USDT 페어만
            }

            let price: f64 = match ticker.last_price.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            // price가 0보다 큰 경우만 추가
            if price <= 0.0 {
                continue;
            }

            let vol_24h_usd: f64 = ticker.turnover24h.parse().unwrap_or(0.0);

            out.push(SpotSnapshot {
                exchange: ExchangeId::Bybit,
                symbol: ticker.symbol,
                currency: Currency::USDT,
                price,
                vol_24h_usd,
                updated_at: now,
            });
        }

        Ok(out)
    }
}
