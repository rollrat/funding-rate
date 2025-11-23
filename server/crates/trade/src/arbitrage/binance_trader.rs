use serde::{Deserialize, Serialize};

use exchanges::{AssetExchange, BinanceClient};
use interface::ExchangeError;

const SPOT_BASE_URL: &str = "https://api.binance.com";
const FUTURES_BASE_URL: &str = "https://fapi.binance.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub symbol: String,
    pub order_id: Option<u64>,
    pub client_order_id: Option<String>,
    pub executed_qty: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

pub struct BinanceTrader {
    spot_client: BinanceClient,
    futures_client: BinanceClient,
}

impl BinanceTrader {
    pub fn new() -> Result<Self, ExchangeError> {
        let spot_client = BinanceClient::with_credentials()
            .map_err(|e| ExchangeError::Other(format!("Failed to create spot client: {}", e)))?;
        let futures_client = BinanceClient::with_credentials()
            .map_err(|e| ExchangeError::Other(format!("Failed to create futures client: {}", e)))?;

        Ok(Self {
            spot_client,
            futures_client,
        })
    }

    /// 스팟 잔고 조회
    pub async fn get_spot_balance(&self, asset: &str) -> Result<f64, ExchangeError> {
        let assets = self
            .spot_client
            .fetch_assets()
            .await
            .map_err(|e| ExchangeError::Other(format!("Failed to fetch spot assets: {}", e)))?;

        let balance = assets
            .iter()
            .find(|a| a.currency == asset)
            .map(|a| a.available)
            .unwrap_or(0.0);

        Ok(balance)
    }

    /// 선물 잔고 조회 (USDT 마진)
    pub async fn get_futures_balance(&self) -> Result<f64, ExchangeError> {
        let api_key = self
            .futures_client
            .api_key
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API key not set".to_string()))?;
        let api_secret = self
            .futures_client
            .api_secret
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API secret not set".to_string()))?;

        let endpoint = "/fapi/v2/balance";
        let timestamp = exchanges::binance::get_timestamp();
        let query_string = format!("timestamp={}", timestamp);
        let signature = exchanges::binance::generate_signature(&query_string, api_secret);

        let url = format!(
            "{}{}?{}&signature={}",
            FUTURES_BASE_URL, endpoint, query_string, signature
        );

        let response = self
            .futures_client
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

        #[derive(Debug, Deserialize)]
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

    /// 심볼에서 베이스 자산 추출 (예: "BTCUSDT" -> "BTC")
    pub fn base_asset_from_symbol(symbol: &str) -> String {
        if symbol.ends_with("USDT") {
            symbol[..symbol.len() - 4].to_string()
        } else if symbol.ends_with("USD") {
            symbol[..symbol.len() - 3].to_string()
        } else {
            symbol.to_string()
        }
    }

    /// 수량을 거래소 규칙에 맞게 조정 (LOT_SIZE)
    pub fn clamp_quantity(symbol: &str, qty: f64) -> f64 {
        // Binance의 일반적인 step size는 0.001 (BTC의 경우)
        // 실제로는 거래소 API에서 exchange info를 가져와야 하지만,
        // 여기서는 간단히 0.001 단위로 반올림
        let step = 0.001;
        (qty / step).floor() * step
    }

    /// 스팟 시장가 주문
    pub async fn place_spot_order(
        &self,
        symbol: &str,
        side: &str, // "BUY" or "SELL"
        quantity: f64,
        test: bool,
    ) -> Result<OrderResponse, ExchangeError> {
        let api_key = self
            .spot_client
            .api_key
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API key not set".to_string()))?;
        let api_secret = self
            .spot_client
            .api_secret
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API secret not set".to_string()))?;

        let endpoint = if test {
            "/api/v3/order/test"
        } else {
            "/api/v3/order"
        };

        let timestamp = exchanges::binance::get_timestamp();
        let qty_str = format!("{:.8}", quantity);
        let query_string = format!(
            "symbol={}&side={}&type=MARKET&quantity={}&timestamp={}",
            symbol, side, qty_str, timestamp
        );
        let signature = exchanges::binance::generate_signature(&query_string, api_secret);

        let url = format!(
            "{}{}?{}&signature={}",
            SPOT_BASE_URL, endpoint, query_string, signature
        );

        let response = self
            .spot_client
            .http
            .post(&url)
            .header("X-MBX-APIKEY", api_key.as_str())
            .send()
            .await
            .map_err(|e| ExchangeError::Other(format!("HTTP error: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return Err(ExchangeError::Other(format!(
                "Spot order API error: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            )));
        }

        let order: OrderResponse = serde_json::from_str(&response_text)
            .map_err(|e| ExchangeError::Other(format!("Failed to parse order response: {}", e)))?;

        Ok(order)
    }

    /// 선물 시장가 주문
    pub async fn place_futures_order(
        &self,
        symbol: &str,
        side: &str, // "BUY" or "SELL"
        quantity: f64,
        reduce_only: bool,
    ) -> Result<OrderResponse, ExchangeError> {
        let api_key = self
            .futures_client
            .api_key
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API key not set".to_string()))?;
        let api_secret = self
            .futures_client
            .api_secret
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API secret not set".to_string()))?;

        let endpoint = "/fapi/v1/order";

        let timestamp = exchanges::binance::get_timestamp();
        let qty_str = format!("{:.8}", quantity);
        let mut query_string = format!(
            "symbol={}&side={}&type=MARKET&quantity={}&timestamp={}",
            symbol, side, qty_str, timestamp
        );

        if reduce_only {
            query_string.push_str("&reduceOnly=true");
        }

        let signature = exchanges::binance::generate_signature(&query_string, api_secret);

        let url = format!(
            "{}{}?{}&signature={}",
            FUTURES_BASE_URL, endpoint, query_string, signature
        );

        let response = self
            .futures_client
            .http
            .post(&url)
            .header("X-MBX-APIKEY", api_key.as_str())
            .send()
            .await
            .map_err(|e| ExchangeError::Other(format!("HTTP error: {}", e)))?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return Err(ExchangeError::Other(format!(
                "Futures order API error: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            )));
        }

        let order: OrderResponse = serde_json::from_str(&response_text)
            .map_err(|e| ExchangeError::Other(format!("Failed to parse order response: {}", e)))?;

        Ok(order)
    }

    /// 선물 마진 타입 및 레버리지 설정
    pub async fn ensure_futures_setup(
        &self,
        symbol: &str,
        leverage: u32,
        isolated: bool,
    ) -> Result<(), ExchangeError> {
        let api_key = self
            .futures_client
            .api_key
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API key not set".to_string()))?;
        let api_secret = self
            .futures_client
            .api_secret
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API secret not set".to_string()))?;

        // 1. 마진 타입 설정
        let endpoint = "/fapi/v1/marginType";
        let timestamp = exchanges::binance::get_timestamp();
        let margin_type = if isolated { "ISOLATED" } else { "CROSS" };
        let query_string = format!(
            "symbol={}&marginType={}&timestamp={}",
            symbol, margin_type, timestamp
        );
        let signature = exchanges::binance::generate_signature(&query_string, api_secret);

        let url = format!(
            "{}{}?{}&signature={}",
            FUTURES_BASE_URL, endpoint, query_string, signature
        );

        let response = self
            .futures_client
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
        let timestamp = exchanges::binance::get_timestamp();
        let query_string = format!(
            "symbol={}&leverage={}&timestamp={}",
            symbol, leverage, timestamp
        );
        let signature = exchanges::binance::generate_signature(&query_string, api_secret);

        let url = format!(
            "{}{}?{}&signature={}",
            FUTURES_BASE_URL, endpoint, query_string, signature
        );

        let response = self
            .futures_client
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

    /// 스팟 현재가 조회
    pub async fn get_spot_price(&self, symbol: &str) -> Result<f64, ExchangeError> {
        let url = format!("{}/api/v3/ticker/price?symbol={}", SPOT_BASE_URL, symbol);

        #[derive(Debug, Deserialize)]
        struct PriceResponse {
            price: String,
        }

        let response: PriceResponse = self
            .spot_client
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ExchangeError::Other(format!("HTTP error: {}", e)))?
            .json()
            .await
            .map_err(|e| ExchangeError::Other(format!("Failed to parse price: {}", e)))?;

        response
            .price
            .parse::<f64>()
            .map_err(|e| ExchangeError::Other(format!("Failed to parse price as f64: {}", e)))
    }

    /// 선물 마크 가격 조회
    pub async fn get_futures_mark_price(&self, symbol: &str) -> Result<f64, ExchangeError> {
        let url = format!(
            "{}/fapi/v1/premiumIndex?symbol={}",
            FUTURES_BASE_URL, symbol
        );

        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct MarkPriceResponse {
            mark_price: String,
        }

        let response: MarkPriceResponse = self
            .futures_client
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ExchangeError::Other(format!("HTTP error: {}", e)))?
            .json()
            .await
            .map_err(|e| ExchangeError::Other(format!("Failed to parse mark price: {}", e)))?;

        response
            .mark_price
            .parse::<f64>()
            .map_err(|e| ExchangeError::Other(format!("Failed to parse mark price as f64: {}", e)))
    }
}
