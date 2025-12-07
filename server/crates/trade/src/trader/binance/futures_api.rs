use std::collections::HashMap;
use std::sync::RwLock;

use exchanges::binance::{generate_signature, get_timestamp};
use exchanges::BinanceClient;
use interface::ExchangeError;

use super::types::{clamp_quantity_with_filter, LotSizeFilter};

const FUTURES_BASE_URL: &str = "https://fapi.binance.com";

/// Binance Futures API: Futures 주문, exchangeInfo, LOT_SIZE 캐시 관리
pub struct BinanceFuturesApi {
    client: BinanceClient,
    lot_size_cache: RwLock<HashMap<String, LotSizeFilter>>,
}

impl BinanceFuturesApi {
    pub fn new(client: BinanceClient) -> Self {
        Self {
            client,
            lot_size_cache: RwLock::new(HashMap::new()),
        }
    }

    /// 선물 exchangeInfo를 로드하여 LOT_SIZE 필터를 캐시에 저장
    pub async fn load_exchange_info(&self) -> Result<(), ExchangeError> {
        let url = format!("{}/fapi/v1/exchangeInfo", FUTURES_BASE_URL);

        let response = self
            .client
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ExchangeError::Other(format!("HTTP error: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return Err(ExchangeError::Other(format!(
                "Futures exchangeInfo API error: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            )));
        }

        let resp: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| ExchangeError::Other(format!("Failed to parse exchangeInfo: {}", e)))?;

        let mut cache = self.lot_size_cache.write().unwrap();
        cache.clear();

        if let Some(symbols) = resp.get("symbols").and_then(|v| v.as_array()) {
            for symbol_info in symbols {
                let symbol = match symbol_info.get("symbol").and_then(|v| v.as_str()) {
                    Some(sym) => sym.to_string(),
                    None => continue,
                };

                if let Some(filters) = symbol_info.get("filters").and_then(|v| v.as_array()) {
                    for filter in filters {
                        let filter_type = filter.get("filterType").and_then(|v| v.as_str());
                        if filter_type == Some("LOT_SIZE") {
                            let min_qty = filter
                                .get("minQty")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);

                            let max_qty = filter
                                .get("maxQty")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(f64::MAX);

                            let step_size = filter
                                .get("stepSize")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(1.0);

                            cache.insert(
                                symbol.clone(),
                                LotSizeFilter {
                                    min_qty,
                                    max_qty,
                                    step_size,
                                },
                            );
                            break;
                        }
                    }
                }
            }
        }

        tracing::info!("Loaded {} futures symbols LOT_SIZE filters", cache.len());
        Ok(())
    }

    /// 선물 심볼의 LOT_SIZE 필터 가져오기
    pub fn get_lot_size(&self, symbol: &str) -> Option<LotSizeFilter> {
        self.lot_size_cache.read().unwrap().get(symbol).copied()
    }

    /// 선물 수량을 거래소 규칙에 맞게 조정 (LOT_SIZE)
    pub fn clamp_quantity(&self, symbol: &str, qty: f64) -> f64 {
        if let Some(filter) = self.get_lot_size(symbol) {
            clamp_quantity_with_filter(filter, qty)
        } else {
            tracing::warn!(
                "LOT_SIZE filter not found for futures symbol: {}. Using original quantity.",
                symbol
            );
            qty
        }
    }

    /// 선물 마진 타입 및 레버리지 설정
    pub async fn ensure_setup(
        &self,
        symbol: &str,
        leverage: u32,
        isolated: bool,
    ) -> Result<(), ExchangeError> {
        let api_key = self
            .client
            .api_key
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API key not set".to_string()))?;
        let api_secret = self
            .client
            .api_secret
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API secret not set".to_string()))?;

        // 1. 마진 타입 설정
        let endpoint = "/fapi/v1/marginType";
        let timestamp = get_timestamp();
        let margin_type = if isolated { "ISOLATED" } else { "CROSS" };
        let query_string = format!(
            "symbol={}&marginType={}&timestamp={}&recvWindow=50000",
            symbol, margin_type, timestamp
        );
        let signature = generate_signature(&query_string, api_secret);

        let url = format!(
            "{}{}?{}&signature={}",
            FUTURES_BASE_URL, endpoint, query_string, signature
        );

        let response = self
            .client
            .http
            .post(&url)
            .header("X-MBX-APIKEY", api_key.as_str())
            .send()
            .await;

        // 마진 타입이 이미 설정되어 있으면 에러가 날 수 있음 (무시)
        if let Ok(resp) = response {
            if !resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                if !text.contains("-4046") {
                    // -4046은 "No need to change margin type" 에러
                    tracing::warn!("Failed to set margin type: {}", text);
                }
            }
        }

        // 2. 레버리지 설정
        let endpoint = "/fapi/v1/leverage";
        let timestamp = get_timestamp();
        let query_string = format!(
            "symbol={}&leverage={}&timestamp={}&recvWindow=50000",
            symbol, leverage, timestamp
        );
        let signature = generate_signature(&query_string, api_secret);

        let url = format!(
            "{}{}?{}&signature={}",
            FUTURES_BASE_URL, endpoint, query_string, signature
        );

        let response = self
            .client
            .http
            .post(&url)
            .header("X-MBX-APIKEY", api_key.as_str())
            .send()
            .await
            .map_err(|e| ExchangeError::Other(format!("HTTP error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let response_text = response.text().await.unwrap_or_default();
            tracing::warn!(
                "Failed to set leverage: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            );
        }

        Ok(())
    }

    /// 선물 잔고 조회 (USDT 마진)
    pub async fn get_balance(&self) -> Result<f64, ExchangeError> {
        let api_key = self
            .client
            .api_key
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API key not set".to_string()))?;
        let api_secret = self
            .client
            .api_secret
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API secret not set".to_string()))?;

        let endpoint = "/fapi/v2/balance";
        let timestamp = get_timestamp();
        let query_string = format!("timestamp={}&recvWindow=50000", timestamp);
        let signature = generate_signature(&query_string, api_secret);

        let url = format!(
            "{}{}?{}&signature={}",
            FUTURES_BASE_URL, endpoint, query_string, signature
        );

        let response = self
            .client
            .http
            .get(&url)
            .header("X-MBX-APIKEY", api_key.as_str())
            .send()
            .await
            .map_err(|e| ExchangeError::Other(format!("HTTP error: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return Err(ExchangeError::Other(format!(
                "Futures balance API error: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            )));
        }

        #[derive(Debug, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FuturesBalance {
            asset: String,
            balance: String,
        }

        let balances: Vec<FuturesBalance> = serde_json::from_str(&response_text)
            .map_err(|e| ExchangeError::Other(format!("Failed to parse balance: {}", e)))?;

        let usdt_balance = balances
            .iter()
            .find(|b| b.asset == "USDT")
            .and_then(|b| b.balance.parse::<f64>().ok())
            .unwrap_or(0.0);

        Ok(usdt_balance)
    }

    pub fn client(&self) -> &BinanceClient {
        &self.client
    }
}

