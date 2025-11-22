use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::model::{ExchangeId, PerpSnapshot};

use super::{ExchangeError, PerpExchange};

const BASE_URL: &str = "https://www.okx.com";

#[derive(Clone)]
pub struct OkxClient {
    http: reqwest::Client,
}

impl OkxClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct OkxResponse<T> {
    code: String,
    msg: String,
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxTicker {
    inst_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    last: String,
    #[serde(default)]
    vol_24h: String,
    #[serde(default)]
    vol_ccy_24h: String, // 24h volume in quote currency (USDT)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxMarkPrice {
    inst_id: String,
    #[serde(default)]
    mark_px: String,
    #[serde(default)]
    funding_rate: String,
    #[serde(default)]
    next_funding_time: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxOpenInterest {
    inst_id: String,
    #[serde(default)]
    oi: String,
    #[serde(default)]
    oi_ccy: String, // open interest in quote currency (USDT)
}

#[async_trait]
impl PerpExchange for OkxClient {
    fn id(&self) -> ExchangeId {
        ExchangeId::Okx
    }

    async fn fetch_all(&self) -> Result<Vec<PerpSnapshot>, ExchangeError> {
        // 1) 티커 정보 (24h 거래량)
        let tickers_url = format!("{BASE_URL}/api/v5/market/tickers?instType=SWAP");
        let tickers_response: OkxResponse<Vec<OkxTicker>> =
            self.http.get(&tickers_url).send().await?.json().await?;

        if tickers_response.code != "0" {
            return Err(ExchangeError::Other(format!(
                "OKX API error (tickers): {} - {}",
                tickers_response.code, tickers_response.msg
            )));
        }

        // 2) 마크 가격 및 펀딩 레이트
        let mark_price_url = format!("{BASE_URL}/api/v5/public/mark-price?instType=SWAP");
        let mark_price_response: OkxResponse<Vec<OkxMarkPrice>> =
            self.http.get(&mark_price_url).send().await?.json().await?;

        if mark_price_response.code != "0" {
            return Err(ExchangeError::Other(format!(
                "OKX API error (mark-price): {} - {}",
                mark_price_response.code, mark_price_response.msg
            )));
        }

        // 3) 오픈 이너스트
        let oi_url = format!("{BASE_URL}/api/v5/public/open-interest?instType=SWAP");
        let oi_response: OkxResponse<Vec<OkxOpenInterest>> =
            self.http.get(&oi_url).send().await?.json().await?;

        if oi_response.code != "0" {
            return Err(ExchangeError::Other(format!(
                "OKX API error (open-interest): {} - {}",
                oi_response.code, oi_response.msg
            )));
        }

        // 맵으로 변환하여 조회 속도 향상
        let mut ticker_map: HashMap<String, OkxTicker> = HashMap::new();
        for ticker in tickers_response.data {
            if ticker.inst_id.ends_with("-USDT-SWAP") {
                ticker_map.insert(ticker.inst_id.clone(), ticker);
            }
        }

        let mut mark_price_map: HashMap<String, OkxMarkPrice> = HashMap::new();
        for mp in mark_price_response.data {
            if mp.inst_id.ends_with("-USDT-SWAP") {
                mark_price_map.insert(mp.inst_id.clone(), mp);
            }
        }

        let mut oi_map: HashMap<String, OkxOpenInterest> = HashMap::new();
        for oi in oi_response.data {
            if oi.inst_id.ends_with("-USDT-SWAP") {
                oi_map.insert(oi.inst_id.clone(), oi);
            }
        }

        let now = Utc::now();
        let mut out = Vec::new();

        // 모든 USDT-SWAP 심볼에 대해 데이터 조합
        for inst_id in ticker_map.keys() {
            let ticker = match ticker_map.get(inst_id) {
                Some(t) => t,
                None => continue,
            };

            let mark_price_data = match mark_price_map.get(inst_id) {
                Some(mp) => mp,
                None => continue,
            };

            let mark_price: f64 = match mark_price_data.mark_px.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let funding_rate: f64 = mark_price_data.funding_rate.parse().unwrap_or(0.0);

            // 오픈 이너스트는 oi_ccy (USDT 기준)를 우선 사용, 없으면 oi * mark_price
            let oi_usd = match oi_map.get(inst_id) {
                Some(oi_data) => {
                    if !oi_data.oi_ccy.is_empty() {
                        oi_data.oi_ccy.parse().unwrap_or(0.0)
                    } else {
                        let oi_contracts: f64 = oi_data.oi.parse().unwrap_or(0.0);
                        oi_contracts * mark_price
                    }
                }
                None => 0.0,
            };

            // 24h 거래량은 volCcy24h (USDT 기준)를 우선 사용, 없으면 vol24h
            let vol_24h_usd: f64 = if !ticker.vol_ccy_24h.is_empty() {
                ticker.vol_ccy_24h.parse().unwrap_or(0.0)
            } else {
                ticker.vol_24h.parse().unwrap_or(0.0)
            };

            let next_funding_time: Option<DateTime<Utc>> =
                if !mark_price_data.next_funding_time.is_empty() {
                    mark_price_data
                        .next_funding_time
                        .parse::<i64>()
                        .ok()
                        .and_then(|ts| DateTime::from_timestamp_millis(ts))
                } else {
                    None
                };

            // OKX는 "BTC-USDT-SWAP" 형식이므로 "BTCUSDT"로 변환
            let symbol = inst_id.replace("-USDT-SWAP", "USDT").replace("-", "");

            out.push(PerpSnapshot {
                exchange: ExchangeId::Okx,
                symbol,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_okx_client_id() {
        let client = OkxClient::new();
        assert_eq!(client.id(), ExchangeId::Okx);
    }

    #[tokio::test]
    async fn test_fetch_all() {
        let client = OkxClient::new();
        let result = client.fetch_all().await;

        match result {
            Ok(snapshots) => {
                // API 호출이 성공했는지 확인
                assert!(!snapshots.is_empty(), "snapshots should not be empty");

                // 모든 스냅샷이 Okx 거래소인지 확인
                for snapshot in &snapshots {
                    assert_eq!(snapshot.exchange, ExchangeId::Okx);
                    assert!(snapshot.symbol.ends_with("USDT"));
                    assert!(snapshot.mark_price > 0.0);
                    assert!(snapshot.oi_usd >= 0.0);
                    assert!(snapshot.vol_24h_usd >= 0.0);
                }

                // 심볼 변환이 올바른지 확인 (예: BTC-USDT-SWAP -> BTCUSDT)
                let btc_snapshot = snapshots.iter().find(|s| s.symbol == "BTCUSDT");
                if btc_snapshot.is_some() {
                    println!("Found BTCUSDT snapshot: {:?}", btc_snapshot);
                }
            }
            Err(e) => {
                // 네트워크 오류 등은 테스트 실패로 간주하지 않음
                // 하지만 API 오류는 확인
                if let ExchangeError::Other(msg) = &e {
                    if msg.contains("OKX API error") {
                        panic!("OKX API error: {}", msg);
                    }
                }
                // 네트워크 오류는 테스트 환경에 따라 실패할 수 있으므로 경고만
                eprintln!("Warning: fetch_all failed: {:?}", e);
            }
        }
    }
}
