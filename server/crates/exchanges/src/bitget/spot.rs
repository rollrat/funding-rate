use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use crate::{BitgetClient, ExchangeError, SpotExchange};
use interface::{Currency, ExchangeId, SpotSnapshot};

const BASE_URL: &str = "https://api.bitget.com";

#[derive(Debug, Deserialize)]
struct BitgetResponse<T> {
    code: String,
    msg: String,
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BitgetSpotTicker {
    symbol: String,
    #[serde(default)]
    close: String, // last price
    #[serde(default)]
    usdt_volume: String, // 24h volume in USDT
}

#[async_trait]
impl SpotExchange for BitgetClient {
    fn id(&self) -> ExchangeId {
        ExchangeId::Bitget
    }

    async fn fetch_all(&self) -> Result<Vec<SpotSnapshot>, ExchangeError> {
        let tickers_url = format!("{BASE_URL}/api/spot/v1/market/tickers");
        let tickers_response: BitgetResponse<Vec<BitgetSpotTicker>> =
            self.http.get(&tickers_url).send().await?.json().await?;

        if tickers_response.code != "00000" {
            return Err(ExchangeError::Other(format!(
                "Bitget API error (spot tickers): {} - {}",
                tickers_response.code, tickers_response.msg
            )));
        }

        let now = Utc::now();
        let mut out = Vec::new();

        for ticker in tickers_response.data {
            if !ticker.symbol.ends_with("USDT") {
                continue; // USDT 페어만
            }

            let price: f64 = match ticker.close.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            // price가 0보다 큰 경우만 추가
            if price <= 0.0 {
                continue;
            }

            let vol_24h_usd: f64 = ticker.usdt_volume.parse().unwrap_or(0.0);

            out.push(SpotSnapshot {
                exchange: ExchangeId::Bitget,
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
