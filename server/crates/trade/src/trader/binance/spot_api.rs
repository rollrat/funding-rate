use std::collections::HashMap;
use std::sync::RwLock;

use exchanges::{AssetExchange, BinanceClient};
use interface::ExchangeError;

use super::types::{clamp_quantity_with_filter, LotSizeFilter};

const SPOT_BASE_URL: &str = "https://api.binance.com";

/// Binance Spot API: Spot 주문, exchangeInfo, LOT_SIZE 캐시 관리
pub struct BinanceSpotApi {
    client: BinanceClient,
    lot_size_cache: RwLock<HashMap<String, LotSizeFilter>>,
}

impl BinanceSpotApi {
    pub fn new(client: BinanceClient) -> Self {
        Self {
            client,
            lot_size_cache: RwLock::new(HashMap::new()),
        }
    }

    /// 스팟 exchangeInfo를 로드하여 LOT_SIZE 필터를 캐시에 저장
    pub async fn load_exchange_info(&self) -> Result<(), ExchangeError> {
        let url = format!("{}/api/v3/exchangeInfo", SPOT_BASE_URL);

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
                "Spot exchangeInfo API error: status {}, response: {}",
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

        tracing::info!("Loaded {} spot symbols LOT_SIZE filters", cache.len());
        Ok(())
    }

    /// 스팟 심볼의 LOT_SIZE 필터 가져오기
    pub fn get_lot_size(&self, symbol: &str) -> Option<LotSizeFilter> {
        self.lot_size_cache.read().unwrap().get(symbol).copied()
    }

    /// 스팟 수량을 거래소 규칙에 맞게 조정 (LOT_SIZE)
    pub fn clamp_quantity(&self, symbol: &str, qty: f64) -> f64 {
        if let Some(filter) = self.get_lot_size(symbol) {
            clamp_quantity_with_filter(filter, qty)
        } else {
            tracing::warn!(
                "LOT_SIZE filter not found for spot symbol: {}. Using original quantity.",
                symbol
            );
            qty
        }
    }

    /// 스팟 잔고 조회
    pub async fn get_balance(&self, asset: &str) -> Result<f64, ExchangeError> {
        let assets = self
            .client
            .fetch_spots()
            .await
            .map_err(|e| ExchangeError::Other(format!("Failed to fetch spot assets: {}", e)))?;

        let balance = assets
            .iter()
            .find(|a| a.currency == asset)
            .map(|a| a.available)
            .unwrap_or(0.0);

        Ok(balance)
    }

    pub fn client(&self) -> &BinanceClient {
        &self.client
    }
}

