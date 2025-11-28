use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use tokio::sync::RwLock;

use interface::{DepositWithdrawalFee, ExchangeId, FeeInfo, MarketType};

use super::super::FeeExchange;
// mod.rs의 BinanceClient를 import하여 FeeExchange trait 구현
use super::{generate_signature, get_api_credentials, get_timestamp, BinanceClient, SAPI_BASE_URL};

/// 입출금 수수료 캐시
static FEE_CACHE: tokio::sync::OnceCell<Arc<RwLock<HashMap<String, DepositWithdrawalFee>>>> =
    tokio::sync::OnceCell::const_new();

/// 거래 수수료 캐시 (symbol -> FeeInfo)
static TRADE_FEE_CACHE: tokio::sync::OnceCell<Arc<RwLock<HashMap<String, FeeInfo>>>> =
    tokio::sync::OnceCell::const_new();

/// 캐시 초기화 (한 번만 실행)
async fn init_fee_cache() -> Arc<RwLock<HashMap<String, DepositWithdrawalFee>>> {
    FEE_CACHE
        .get_or_init(|| async { Arc::new(RwLock::new(HashMap::new())) })
        .await
        .clone()
}

/// 거래 수수료 캐시 초기화
async fn init_trade_fee_cache() -> Arc<RwLock<HashMap<String, FeeInfo>>> {
    TRADE_FEE_CACHE
        .get_or_init(|| async { Arc::new(RwLock::new(HashMap::new())) })
        .await
        .clone()
}

/// Binance 입출금 수수료 API 응답 (getall 엔드포인트)
/// 한 번의 호출로 모든 코인의 네트워크 정보까지 포함해서 반환
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceCoinInfo {
    coin: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    deposit_all_enable: Option<bool>,
    #[serde(default)]
    withdraw_all_enable: Option<bool>,
    #[serde(default, rename = "networkList")]
    network_list: Vec<BinanceNetwork>,
    // 기타 필드들(free, locked 등)은 무시
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceNetwork {
    network: String,
    #[serde(default)]
    deposit_enable: Option<bool>,
    #[serde(default)]
    withdraw_enable: Option<bool>,
    #[serde(default)]
    deposit_tip: Option<String>,
    #[serde(default, rename = "withdrawFee")]
    withdraw_fee: String,
    #[serde(default, rename = "withdrawMin")]
    min_withdraw_amount: Option<String>,
    #[serde(default, rename = "withdrawMax")]
    max_withdraw_amount: Option<String>,
}

impl BinanceClient {
    /// 입출금 수수료 캐시 초기화 및 업데이트
    pub async fn refresh_deposit_withdrawal_fees(
        &self,
    ) -> Result<HashMap<String, DepositWithdrawalFee>, super::super::ExchangeError> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            super::super::ExchangeError::Other(
                "API key not set. Use BinanceClient::with_credentials()".to_string(),
            )
        })?;
        let api_secret = self.api_secret.as_ref().ok_or_else(|| {
            super::super::ExchangeError::Other(
                "API secret not set. Use BinanceClient::with_credentials()".to_string(),
            )
        })?;

        // GET /sapi/v1/capital/config/getall
        // 한 번의 호출로 모든 코인의 네트워크 정보까지 포함해서 반환
        let endpoint = "/sapi/v1/capital/config/getall";
        let timestamp = get_timestamp();
        let query_string = format!("timestamp={}&recvWindow=50000", timestamp);
        let signature = generate_signature(&query_string, &api_secret);
        let url = format!(
            "{}{}?{}&signature={}",
            SAPI_BASE_URL, endpoint, query_string, signature
        );

        let response = self
            .http
            .get(&url)
            .header("X-MBX-APIKEY", api_key.as_str())
            .send()
            .await?;

        // response.text()는 한 번만 호출 (바디를 소비하므로)
        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return Err(super::super::ExchangeError::Other(format!(
                "Failed to fetch fee API: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            )));
        }

        // getall 엔드포인트는 모든 코인의 네트워크 정보까지 포함해서 반환
        let coin_infos: Vec<BinanceCoinInfo> =
            serde_json::from_str(&response_text).map_err(|e| {
                super::super::ExchangeError::Other(format!(
                    "Failed to parse coin config: {}, response: {}",
                    e,
                    response_text.chars().take(200).collect::<String>()
                ))
            })?;

        let mut fees = HashMap::new();
        let now = Utc::now();

        // 주요 코인들만 필터링 (선택사항 - 전체 코인을 처리하려면 이 필터 제거)
        let major_coins: std::collections::HashSet<&str> = [
            "BTC", "ETH", "USDT", "BNB", "SOL", "XRP", "ADA", "DOGE", "DOT", "MATIC", "LINK",
            "UNI", "LTC", "AVAX",
        ]
        .iter()
        .cloned()
        .collect();

        for info in coin_infos.iter() {
            // 주요 코인만 필터링 (전체 코인을 원하면 이 조건 제거)
            if !major_coins.contains(info.coin.as_str()) {
                continue;
            }

            // 여러 네트워크가 있는 경우, 최소 출금 수수료를 가진 네트워크 사용
            let mut best_network: Option<&BinanceNetwork> = None;
            let mut min_withdraw_fee = f64::MAX;

            for network in &info.network_list {
                // withdraw_enable이 true인 네트워크만 고려
                if network.withdraw_enable.unwrap_or(false) {
                    if let Ok(fee) = network.withdraw_fee.parse::<f64>() {
                        if fee < min_withdraw_fee {
                            min_withdraw_fee = fee;
                            best_network = Some(network);
                        }
                    }
                }
            }

            if let Some(network) = best_network {
                let currency = info.coin.to_uppercase();
                let deposit_fee = 0.0; // Binance는 입금 수수료가 없음
                let withdrawal_fee = network.withdraw_fee.parse::<f64>().unwrap_or(0.0);

                fees.insert(
                    currency.clone(),
                    DepositWithdrawalFee {
                        currency,
                        deposit_fee,
                        withdrawal_fee,
                        updated_at: now,
                    },
                );
            }
        }

        tracing::info!(
            "Parsed {} deposit/withdrawal fees from Binance API",
            fees.len()
        );

        // 캐시 업데이트
        let cache = init_fee_cache().await;
        *cache.write().await = fees.clone();

        Ok(fees)
    }

    /// 입출금 수수료 캐시 가져오기 (없으면 초기화)
    async fn get_fee_cache(
        &self,
    ) -> Result<Arc<RwLock<HashMap<String, DepositWithdrawalFee>>>, super::super::ExchangeError>
    {
        let cache = init_fee_cache().await;

        // 캐시가 비어있으면 초기화
        if cache.read().await.is_empty() {
            self.refresh_deposit_withdrawal_fees().await?;
        }

        Ok(cache)
    }

    /// 거래 수수료 캐시 초기화 및 업데이트
    pub async fn refresh_trade_fees(
        &self,
    ) -> Result<HashMap<String, FeeInfo>, super::super::ExchangeError> {
        let api_key = self.api_key.as_ref().ok_or_else(|| {
            super::super::ExchangeError::Other(
                "API key not set. Use BinanceClient::with_credentials()".to_string(),
            )
        })?;
        let api_secret = self.api_secret.as_ref().ok_or_else(|| {
            super::super::ExchangeError::Other(
                "API secret not set. Use BinanceClient::with_credentials()".to_string(),
            )
        })?;

        // GET /sapi/v1/asset/tradeFee
        let endpoint = "/sapi/v1/asset/tradeFee";
        let timestamp = get_timestamp();
        let query_string = format!("timestamp={}&recvWindow=50000", timestamp);
        let signature = generate_signature(&query_string, &api_secret);
        let url = format!(
            "{}{}?{}&signature={}",
            SAPI_BASE_URL, endpoint, query_string, signature
        );

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct TradeFeeResponse {
            symbol: String,
            maker_commission: String,
            taker_commission: String,
        }

        let response = self
            .http
            .get(&url)
            .header("X-MBX-APIKEY", api_key.as_str())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let response_text = response.text().await?;
            return Err(super::super::ExchangeError::Other(format!(
                "Failed to fetch trade fee API: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            )));
        }

        let trade_fees: Vec<TradeFeeResponse> = response.json().await?;

        let mut fees = HashMap::new();
        for fee_response in trade_fees {
            let maker = fee_response.maker_commission.parse::<f64>().unwrap_or(0.0);
            let taker = fee_response.taker_commission.parse::<f64>().unwrap_or(0.0);

            fees.insert(fee_response.symbol.clone(), FeeInfo::new(maker, taker));
        }

        tracing::info!("Parsed {} trade fees from Binance API", fees.len());

        // 캐시 업데이트
        let cache = init_trade_fee_cache().await;
        *cache.write().await = fees.clone();

        Ok(fees)
    }

    /// 거래 수수료 캐시 가져오기 (없으면 초기화)
    async fn get_trade_fee_cache(
        &self,
    ) -> Result<Arc<RwLock<HashMap<String, FeeInfo>>>, super::super::ExchangeError> {
        let cache = init_trade_fee_cache().await;

        // 캐시가 비어있으면 초기화
        if cache.read().await.is_empty() {
            self.refresh_trade_fees().await?;
        }

        Ok(cache)
    }

    /// 특정 심볼의 거래 수수료 조회
    pub async fn get_trade_fee_for_symbol(
        &self,
        symbol: &str,
    ) -> Result<FeeInfo, super::super::ExchangeError> {
        let cache = self.get_trade_fee_cache().await?;
        let cache_guard = cache.read().await;

        // 심볼을 대문자로 변환 (예: "BTCUSDT", "BTC-USDT" -> "BTCUSDT")
        let normalized_symbol = symbol.replace("-", "").to_uppercase();

        cache_guard.get(&normalized_symbol).cloned().ok_or_else(|| {
            super::super::ExchangeError::Other(format!(
                "Trade fee not found for symbol: {}",
                symbol
            ))
        })
    }
}

#[async_trait]
impl FeeExchange for BinanceClient {
    fn id(&self) -> ExchangeId {
        ExchangeId::Binance
    }

    fn get_fee(&self, market_type: MarketType) -> FeeInfo {
        // 동기 함수이므로 기본값 반환
        // 실제 거래 수수료는 get_trade_fee_for_symbol()을 사용하여 비동기로 조회
        // 일반적인 Binance 거래 수수료: 0.1% (0.001)
        match market_type {
            MarketType::KRW => FeeInfo::new(0.001, 0.001), // 0.1% 메이커, 테이커
            MarketType::USDT => FeeInfo::new(0.001, 0.001), // 0.1% 메이커, 테이커
            MarketType::BTC => FeeInfo::new(0.001, 0.001), // 0.1% 메이커, 테이커
            MarketType::Other(_) => FeeInfo::new(0.001, 0.001), // 기본값: 0.1%
        }
    }

    async fn get_deposit_withdrawal_fee(
        &self,
        currency: &str,
    ) -> Result<DepositWithdrawalFee, super::super::ExchangeError> {
        let cache = self.get_fee_cache().await?;
        let cache_guard = cache.read().await;

        let currency_upper = currency.to_uppercase();
        cache_guard.get(&currency_upper).cloned().ok_or_else(|| {
            super::super::ExchangeError::Other(format!(
                "Fee information not found for currency: {}",
                currency
            ))
        })
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

    fn handle_api_error(e: &super::super::ExchangeError) {
        match e {
            super::super::ExchangeError::Http(reqwest_err) => {
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
            super::super::ExchangeError::Other(msg) => {
                println!("기타 오류: {}", msg);
            }
        }
    }

    #[tokio::test]
    async fn test_refresh_deposit_withdrawal_fees() {
        skip_if_no_credentials();

        let client = BinanceClient::with_credentials()
            .map_err(|e| {
                println!("BinanceClient 생성 실패: {:?}", e);
                return;
            })
            .unwrap();
        let result = client.refresh_deposit_withdrawal_fees().await;

        match result {
            Ok(fees) => {
                assert!(!fees.is_empty(), "Should fetch at least one fee");
                println!(
                    "Successfully fetched {} deposit/withdrawal fees",
                    fees.len()
                );

                // 처음 10개 출력
                for (currency, fee) in fees.iter().take(10) {
                    println!(
                        "{} - Deposit: {}, Withdrawal: {}",
                        currency, fee.deposit_fee, fee.withdrawal_fee
                    );
                }

                // BTC 수수료 확인
                if let Some(btc_fee) = fees.get("BTC") {
                    println!(
                        "\nBTC - Deposit: {}, Withdrawal: {}",
                        btc_fee.deposit_fee, btc_fee.withdrawal_fee
                    );
                }
            }
            Err(e) => {
                handle_api_error(&e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_deposit_withdrawal_fee() {
        skip_if_no_credentials();

        let client = BinanceClient::with_credentials()
            .map_err(|e| {
                println!("BinanceClient 생성 실패: {:?}", e);
                return;
            })
            .unwrap();

        // 먼저 캐시 초기화
        let _ = client.refresh_deposit_withdrawal_fees().await;

        // BTC 수수료 조회
        let result = client.get_deposit_withdrawal_fee("BTC").await;

        match result {
            Ok(fee) => {
                assert_eq!(fee.currency, "BTC");
                assert!(fee.deposit_fee >= 0.0);
                assert!(fee.withdrawal_fee >= 0.0);
                println!(
                    "BTC fees - Deposit: {}, Withdrawal: {}",
                    fee.deposit_fee, fee.withdrawal_fee
                );
            }
            Err(e) => {
                handle_api_error(&e);
            }
        }
    }
}
