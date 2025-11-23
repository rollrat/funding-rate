use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use crate::{BinanceClient, ExchangeError, SpotExchange};
use interface::{Currency, ExchangeId, SpotSnapshot};

const SPOT_BASE_URL: &str = "https://api.binance.com";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceSpotTicker24h {
    symbol: String,
    last_price: String, // Binance Spot API는 lastPrice 필드를 사용
    quote_volume: String,
}

#[async_trait]
impl SpotExchange for BinanceClient {
    fn id(&self) -> ExchangeId {
        ExchangeId::Binance
    }

    async fn fetch_all(&self) -> Result<Vec<SpotSnapshot>, ExchangeError> {
        let tickers: Vec<BinanceSpotTicker24h> = self
            .http
            .get(format!("{SPOT_BASE_URL}/api/v3/ticker/24hr"))
            .send()
            .await?
            .json()
            .await?;

        let now = Utc::now();
        let mut out = Vec::new();

        for ticker in tickers {
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

            let vol_24h_usd: f64 = ticker.quote_volume.parse().unwrap_or(0.0);

            out.push(SpotSnapshot {
                exchange: ExchangeId::Binance,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_spot_client_id() {
        let client = BinanceClient::new();
        assert_eq!(client.id(), ExchangeId::Binance);
    }

    #[tokio::test]
    async fn test_fetch_all_binance_spot() {
        let client = BinanceClient::new();
        let result = client.fetch_all().await;

        match result {
            Ok(snapshots) => {
                // API 호출이 성공했는지 확인
                assert!(!snapshots.is_empty(), "snapshots should not be empty");

                // 모든 스냅샷이 Binance 거래소인지 확인
                for snapshot in &snapshots {
                    println!("Binance spot snapshot: {:?}", snapshot);
                    assert_eq!(snapshot.exchange, ExchangeId::Binance);
                    assert!(snapshot.symbol.ends_with("USDT"));
                    assert!(snapshot.price > 0.0);
                    assert!(snapshot.vol_24h_usd >= 0.0);
                }

                // 심볼이 올바른지 확인 (예: BTCUSDT)
                let btc_snapshot = snapshots.iter().find(|s| s.symbol == "BTCUSDT");
                if let Some(btc) = btc_snapshot {
                    println!(
                        "Found BTCUSDT spot snapshot: price={}, vol_24h_usd={}",
                        btc.price, btc.vol_24h_usd
                    );
                }
            }
            Err(e) => {
                // 네트워크 오류 등은 테스트 실패로 간주하지 않음
                // 하지만 API 오류는 확인
                if let ExchangeError::Other(msg) = &e {
                    if msg.contains("Binance API error") {
                        panic!("Binance API error: {}", msg);
                    }
                }
                // 네트워크 오류는 테스트 환경에 따라 실패할 수 있으므로 경고만
                eprintln!("Warning: fetch_all failed: {:?}", e);

                panic!();
            }
        }
    }
}
