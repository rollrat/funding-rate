use async_trait::async_trait;
use tracing::info;

use exchanges::BinanceClient;
use exchanges::binance::{generate_signature, get_timestamp};
use interface::ExchangeError;

use super::types::{OrderResponse, PlaceFuturesOrderOptions, PlaceOrderOptions};

const SPOT_BASE_URL: &str = "https://api.binance.com";
const FUTURES_BASE_URL: &str = "https://fapi.binance.com";

/// BinanceTrader가 의존하는 주문 클라이언트 트레이트. 나중에 WebSocket 기반 구현체를 추가할 수 있다.
#[async_trait]
pub trait BinanceOrderClient: Send + Sync {
    async fn place_spot_order(
        &self,
        symbol: &str,
        side: &str,
        qty: f64,
        price: Option<f64>,
        options: PlaceOrderOptions,
    ) -> Result<OrderResponse, ExchangeError>;

    async fn place_futures_order(
        &self,
        symbol: &str,
        side: &str,
        qty: f64,
        price: Option<f64>,
        options: PlaceFuturesOrderOptions,
    ) -> Result<OrderResponse, ExchangeError>;

    async fn cancel_spot_order(&self, symbol: &str, order_id: &str) -> Result<(), ExchangeError>;

    async fn cancel_futures_order(&self, symbol: &str, order_id: &str)
    -> Result<(), ExchangeError>;
}

/// HTTP 기반으로 Binance Spot/Futures 주문을 보내는 구현체
pub struct HttpBinanceOrderClient {
    spot_client: BinanceClient,
    futures_client: BinanceClient,
}

impl HttpBinanceOrderClient {
    pub fn new(spot_client: BinanceClient, futures_client: BinanceClient) -> Self {
        Self {
            spot_client,
            futures_client,
        }
    }
}

#[async_trait]
impl BinanceOrderClient for HttpBinanceOrderClient {
    async fn place_spot_order(
        &self,
        symbol: &str,
        side: &str,
        qty: f64,
        _price: Option<f64>,
        options: PlaceOrderOptions,
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

        let endpoint = if options.test {
            "/api/v3/order/test"
        } else {
            "/api/v3/order"
        };

        let timestamp = get_timestamp();
        let qty_str = format!("{:.8}", qty);
        let query_string = format!(
            "symbol={}&side={}&type=MARKET&quantity={}&timestamp={}&recvWindow=50000",
            symbol, side, qty_str, timestamp
        );
        info!("place_spot_order query_string: {}", query_string);
        let signature = generate_signature(&query_string, api_secret);

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

        info!("place_spot_order response: {}", response_text);

        if !status.is_success() {
            return Err(ExchangeError::Other(format!(
                "Spot order API error: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            )));
        }

        let order: OrderResponse = serde_json::from_str(&response_text)
            .map_err(|e| ExchangeError::Other(format!("Failed to parse order response: {}", e)))?;

        // 거래 기록 저장 (test 모드가 아닐 때만)
        if !options.test {
            crate::record::save_trade_record_spot_order(
                "binance",
                symbol,
                side,
                qty,
                &query_string,
                &order,
                false,
            )
            .await;
        }

        Ok(order)
    }

    async fn place_futures_order(
        &self,
        symbol: &str,
        side: &str,
        qty: f64,
        _price: Option<f64>,
        options: PlaceFuturesOrderOptions,
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

        let timestamp = get_timestamp();
        let qty_str = format!("{:.8}", qty);
        let mut query_string = format!(
            "symbol={}&side={}&type=MARKET&quantity={}&timestamp={}&recvWindow=50000",
            symbol, side, qty_str, timestamp
        );

        info!("place_futures_order query_string: {}", query_string);

        if options.reduce_only {
            query_string.push_str("&reduceOnly=true");
        }

        let signature = generate_signature(&query_string, api_secret);

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

        info!("place_futures_order response: {}", response_text);

        if !status.is_success() {
            return Err(ExchangeError::Other(format!(
                "Futures order API error: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            )));
        }

        let order: OrderResponse = serde_json::from_str(&response_text)
            .map_err(|e| ExchangeError::Other(format!("Failed to parse order response: {}", e)))?;

        // 거래 기록 저장
        crate::record::save_trade_record_futures_order(
            "binance",
            symbol,
            side,
            qty,
            &query_string,
            &order,
            options.reduce_only,
            false, // is_liquidation: reduce_only는 정상 포지션 청산이지 강제 청산이 아님
        )
        .await;

        Ok(order)
    }

    async fn cancel_spot_order(&self, _symbol: &str, _order_id: &str) -> Result<(), ExchangeError> {
        // TODO: 구현 필요
        Err(ExchangeError::Other("Not implemented".to_string()))
    }

    async fn cancel_futures_order(
        &self,
        _symbol: &str,
        _order_id: &str,
    ) -> Result<(), ExchangeError> {
        // TODO: 구현 필요
        Err(ExchangeError::Other("Not implemented".to_string()))
    }
}
