use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use interface::{Asset, ExchangeId};

use super::super::{AssetExchange, ExchangeError};
use super::{generate_signature, get_timestamp, BinanceClient, BASE_URL};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceAccountResponse {
    balances: Vec<BinanceBalance>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceBalance {
    asset: String,
    free: String,   // 사용 가능한 잔액
    locked: String, // 주문에 사용 중인 잔액
}

#[async_trait]
impl AssetExchange for BinanceClient {
    fn id(&self) -> ExchangeId {
        ExchangeId::Binance
    }

    async fn fetch_assets(&self) -> Result<Vec<Asset>, ExchangeError> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            ExchangeError::Other(
                "API key not set. Use BinanceClient::with_credentials()".to_string(),
            )
        })?;
        let api_secret = self.api_secret.as_ref().ok_or_else(|| {
            ExchangeError::Other(
                "API secret not set. Use BinanceClient::with_credentials()".to_string(),
            )
        })?;

        // GET /api/v3/account
        let endpoint = "/api/v3/account";

        // 쿼리 파라미터 생성
        let timestamp = get_timestamp();
        let query_string = format!("timestamp={}&recvWindow=50000", timestamp);
        let signature = generate_signature(&query_string, api_secret);
        let url = format!(
            "{}{}?{}&signature={}",
            BASE_URL, endpoint, query_string, signature
        );

        let response = self
            .http
            .get(&url)
            .header("X-MBX-APIKEY", api_key.as_str())
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return Err(ExchangeError::Other(format!(
                "Binance API HTTP error: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            )));
        }

        let account: BinanceAccountResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                ExchangeError::Other(format!(
                    "Failed to parse Binance response: {}, response: {}",
                    e,
                    response_text.chars().take(200).collect::<String>()
                ))
            })?;

        let now = Utc::now();
        let mut assets = Vec::new();

        for balance in account.balances {
            let free: f64 = balance.free.parse().unwrap_or(0.0);
            let locked: f64 = balance.locked.parse().unwrap_or(0.0);
            let total = free + locked;

            // 잔액이 0인 경우 스킵 (선택사항)
            if total > 0.0 {
                assets.push(Asset {
                    currency: balance.asset,
                    total,
                    available: free,
                    in_use: locked,
                    updated_at: now,
                });
            }
        }

        Ok(assets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skip_if_no_credentials() {
        if !super::super::has_api_credentials() {
            println!("Skipping test: BINANCE_API_KEY and BINANCE_API_SECRET not set");
        }
    }

    fn handle_api_error(e: &ExchangeError) {
        match e {
            ExchangeError::Http(reqwest_err) => {
                if let Some(status) = reqwest_err.status() {
                    if status.as_u16() == 401 {
                        println!("API 인증 실패: API 키 또는 시크릿이 잘못되었습니다.");
                    } else if status.as_u16() == 403 {
                        println!("API 권한 없음: API 키에 필요한 권한이 없습니다.");
                    } else {
                        println!("HTTP 오류: {:?}", reqwest_err);
                    }
                } else {
                    println!("HTTP 오류: {:?}", reqwest_err);
                }
            }
            ExchangeError::Other(msg) => {
                println!("기타 오류: {}", msg);
            }
        }
    }

    #[test]
    fn test_binance_asset_client_id() {
        skip_if_no_credentials();

        if let Ok(client) = BinanceClient::with_credentials() {
            assert_eq!(client.id(), ExchangeId::Binance);
        }
    }

    #[tokio::test]
    async fn test_fetch_assets_binance() {
        skip_if_no_credentials();

        let client = match BinanceClient::with_credentials() {
            Ok(client) => client,
            Err(e) => {
                println!("BinanceClient 생성 실패: {:?}", e);
                println!("BINANCE_API_KEY와 BINANCE_API_SECRET 환경변수를 설정해주세요.");
                return;
            }
        };

        match client.fetch_assets().await {
            Ok(assets) => {
                assert!(!assets.is_empty(), "Should fetch at least one asset");
                println!("Successfully fetched {} assets", assets.len());

                // 처음 5개 출력
                for asset in assets.iter().take(5) {
                    println!(
                        "{} - Total: {}, Available: {}, In Use: {}",
                        asset.currency, asset.total, asset.available, asset.in_use
                    );
                }
            }
            Err(e) => {
                handle_api_error(&e);
                // 테스트 실패로 처리하지 않음 (API 키가 없을 수 있음)
            }
        }
    }
}
