use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use crate::{ExchangeError, OkxClient, SpotExchange};
use interface::{Currency, ExchangeId, SpotSnapshot};

const BASE_URL: &str = "https://www.okx.com";

#[derive(Debug, Deserialize)]
struct OkxResponse<T> {
    code: String,
    msg: String,
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxSpotTicker {
    inst_id: String,
    #[serde(default)]
    last: String,
    #[serde(default)]
    vol_ccy_24h: String, // 24h volume in quote currency (USDT)
}

#[async_trait]
impl SpotExchange for OkxClient {
    fn id(&self) -> ExchangeId {
        ExchangeId::Okx
    }

    async fn fetch_all(&self) -> Result<Vec<SpotSnapshot>, ExchangeError> {
        let tickers_url = format!("{BASE_URL}/api/v5/market/tickers?instType=SPOT");
        let tickers_response: OkxResponse<Vec<OkxSpotTicker>> =
            self.http.get(&tickers_url).send().await?.json().await?;

        if tickers_response.code != "0" {
            return Err(ExchangeError::Other(format!(
                "OKX API error (spot tickers): {} - {}",
                tickers_response.code, tickers_response.msg
            )));
        }

        let now = Utc::now();
        let mut out = Vec::new();

        for ticker in tickers_response.data {
            if !ticker.inst_id.ends_with("-USDT") {
                continue; // USDT 페어만
            }

            let price: f64 = match ticker.last.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            // price가 0보다 큰 경우만 추가
            if price <= 0.0 {
                continue;
            }

            let vol_24h_usd: f64 = ticker.vol_ccy_24h.parse().unwrap_or(0.0);

            // OKX는 "BTC-USDT" 형식이므로 "BTCUSDT"로 변환
            let symbol = ticker.inst_id.replace("-USDT", "USDT").replace("-", "");

            out.push(SpotSnapshot {
                exchange: ExchangeId::Okx,
                symbol,
                currency: Currency::USDT,
                price,
                vol_24h_usd,
                updated_at: now,
            });
        }

        Ok(out)
    }
}
