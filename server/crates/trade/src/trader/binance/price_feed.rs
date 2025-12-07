use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::StreamExt;
use tracing::{info, warn};

use exchanges::BinanceClient;
use interface::ExchangeError;

use super::types::PriceState;

const SPOT_BASE_URL: &str = "https://api.binance.com";
const FUTURES_BASE_URL: &str = "https://fapi.binance.com";
const SPOT_WS_URL: &str = "wss://stream.binance.com:9443/ws";
const FUTURES_WS_URL: &str = "wss://fstream.binance.com/ws";

/// Binance Price Feed: WebSocket 가격 스트림 관리
pub struct BinancePriceFeed {
    price_state: Arc<TokioRwLock<HashMap<String, PriceState>>>,
    spot_client: BinanceClient,
    futures_client: BinanceClient,
}

impl BinancePriceFeed {
    pub fn new(spot_client: BinanceClient, futures_client: BinanceClient) -> Self {
        Self {
            price_state: Arc::new(TokioRwLock::new(HashMap::new())),
            spot_client,
            futures_client,
        }
    }

    /// 특정 심볼에 대한 WebSocket 리스너 시작
    /// 스팟 ticker와 선물 markPrice를 동시에 구독
    pub fn start_symbol(&self, symbol: &str) {
        let price_state = Arc::clone(&self.price_state);

        // 스팟 ticker WebSocket
        let spot_state = Arc::clone(&price_state);
        let spot_symbol = symbol.to_string();
        tokio::spawn(async move {
            Self::start_spot_websocket(&spot_symbol, spot_state).await;
        });

        // 선물 markPrice WebSocket
        let fut_state = Arc::clone(&price_state);
        let fut_symbol = symbol.to_string();
        tokio::spawn(async move {
            Self::start_futures_websocket(&fut_symbol, fut_state).await;
        });

        info!("WebSocket 리스너 시작: {}", symbol);
    }

    /// 스팟 현재가 조회 (메모리에서 읽기, 없으면 HTTP 폴백)
    pub async fn get_spot_price(&self, symbol: &str) -> Result<f64, ExchangeError> {
        // 먼저 메모리에서 읽기 시도
        {
            let state_map = self.price_state.read().await;
            if let Some(price_state) = state_map.get(symbol) {
                if let Some(price) = price_state.spot_price {
                    return Ok(price);
                }
            }
        }

        // 메모리에 없으면 HTTP 폴백
        warn!(
            "WebSocket에서 스팟 가격을 찾을 수 없어 HTTP로 조회합니다 (symbol: {})",
            symbol
        );
        let url = format!("{}/api/v3/ticker/price?symbol={}", SPOT_BASE_URL, symbol);

        #[derive(Debug, serde::Deserialize)]
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

        let price = response
            .price
            .parse::<f64>()
            .map_err(|e| ExchangeError::Other(format!("Failed to parse price as f64: {}", e)))?;

        // HTTP로 가져온 가격도 메모리에 저장
        {
            let mut state_map = self.price_state.write().await;
            let price_state = state_map
                .entry(symbol.to_string())
                .or_insert_with(PriceState::default);
            price_state.spot_price = Some(price);
            price_state.last_updated = Some(std::time::SystemTime::now());
        }

        Ok(price)
    }

    /// 선물 마크 가격 조회 (메모리에서 읽기, 없으면 HTTP 폴백)
    pub async fn get_futures_mark_price(&self, symbol: &str) -> Result<f64, ExchangeError> {
        // 먼저 메모리에서 읽기 시도
        {
            let state_map = self.price_state.read().await;
            if let Some(price_state) = state_map.get(symbol) {
                if let Some(price) = price_state.futures_mark_price {
                    return Ok(price);
                }
            }
        }

        // 메모리에 없으면 HTTP 폴백
        warn!(
            "WebSocket에서 선물 마크 가격을 찾을 수 없어 HTTP로 조회합니다 (symbol: {})",
            symbol
        );
        let url = format!(
            "{}/fapi/v1/premiumIndex?symbol={}",
            FUTURES_BASE_URL, symbol
        );

        #[derive(Debug, serde::Deserialize)]
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

        let price = response.mark_price.parse::<f64>().map_err(|e| {
            ExchangeError::Other(format!("Failed to parse mark price as f64: {}", e))
        })?;

        // HTTP로 가져온 가격도 메모리에 저장
        {
            let mut state_map = self.price_state.write().await;
            let price_state = state_map
                .entry(symbol.to_string())
                .or_insert_with(PriceState::default);
            price_state.futures_mark_price = Some(price);
            price_state.last_updated = Some(std::time::SystemTime::now());
        }

        Ok(price)
    }

    /// 스팟 ticker WebSocket 연결 및 수신
    async fn start_spot_websocket(
        symbol: &str,
        state: Arc<TokioRwLock<HashMap<String, PriceState>>>,
    ) {
        let symbol_lower = symbol.to_lowercase();
        let stream_name = format!("{}@ticker", symbol_lower);
        let url = format!("{}/{}", SPOT_WS_URL, stream_name);

        loop {
            match Self::connect_spot_websocket(&url, symbol, state.clone()).await {
                Ok(_) => {
                    warn!(
                        "스팟 WebSocket 연결이 종료되었습니다. 재연결 시도... (symbol: {})",
                        symbol
                    );
                }
                Err(e) => {
                    warn!(
                        "스팟 WebSocket 오류: {:?}. 재연결 시도... (symbol: {})",
                        e, symbol
                    );
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    /// 선물 markPrice WebSocket 연결 및 수신
    async fn start_futures_websocket(
        symbol: &str,
        state: Arc<TokioRwLock<HashMap<String, PriceState>>>,
    ) {
        let symbol_lower = symbol.to_lowercase();
        let stream_name = format!("{}@markPrice", symbol_lower);
        let url = format!("{}/{}", FUTURES_WS_URL, stream_name);

        loop {
            match Self::connect_futures_websocket(&url, symbol, state.clone()).await {
                Ok(_) => {
                    warn!(
                        "선물 WebSocket 연결이 종료되었습니다. 재연결 시도... (symbol: {})",
                        symbol
                    );
                }
                Err(e) => {
                    warn!(
                        "선물 WebSocket 오류: {:?}. 재연결 시도... (symbol: {})",
                        e, symbol
                    );
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    /// 스팟 ticker WebSocket 연결 및 메시지 처리
    async fn connect_spot_websocket(
        url: &str,
        symbol: &str,
        state: Arc<TokioRwLock<HashMap<String, PriceState>>>,
    ) -> Result<(), ExchangeError> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| ExchangeError::Other(format!("WebSocket 연결 실패: {}", e)))?;
        let (_write, mut read) = ws_stream.split();

        info!("스팟 WebSocket 연결 성공: {} (symbol: {})", url, symbol);

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) =
                        Self::handle_spot_ticker_message(&text, symbol, state.clone()).await
                    {
                        warn!("스팟 ticker 메시지 처리 오류: {:?}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    warn!("스팟 WebSocket 연결이 닫혔습니다 (symbol: {})", symbol);
                    break;
                }
                Err(e) => {
                    warn!(
                        "스팟 WebSocket 메시지 수신 오류: {:?} (symbol: {})",
                        e, symbol
                    );
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 선물 markPrice WebSocket 연결 및 메시지 처리
    async fn connect_futures_websocket(
        url: &str,
        symbol: &str,
        state: Arc<TokioRwLock<HashMap<String, PriceState>>>,
    ) -> Result<(), ExchangeError> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| ExchangeError::Other(format!("WebSocket 연결 실패: {}", e)))?;
        let (_write, mut read) = ws_stream.split();

        info!("선물 WebSocket 연결 성공: {} (symbol: {})", url, symbol);

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) =
                        Self::handle_futures_mark_price_message(&text, symbol, state.clone()).await
                    {
                        warn!("선물 markPrice 메시지 처리 오류: {:?}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    warn!("선물 WebSocket 연결이 닫혔습니다 (symbol: {})", symbol);
                    break;
                }
                Err(e) => {
                    warn!(
                        "선물 WebSocket 메시지 수신 오류: {:?} (symbol: {})",
                        e, symbol
                    );
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 스팟 ticker 메시지 처리
    async fn handle_spot_ticker_message(
        text: &str,
        symbol: &str,
        state: Arc<TokioRwLock<HashMap<String, PriceState>>>,
    ) -> Result<(), ExchangeError> {
        #[derive(Debug, serde::Deserialize)]
        struct SpotTicker {
            #[serde(rename = "s")]
            symbol: String,
            #[serde(rename = "c")]
            last_price: String,
        }

        let ticker: SpotTicker = serde_json::from_str(text).map_err(|e| {
            ExchangeError::Other(format!("스팟 ticker 파싱 실패: {} (text: {})", e, text))
        })?;

        if ticker.symbol != symbol {
            return Ok(());
        }

        let price: f64 = ticker.last_price.parse().map_err(|e| {
            ExchangeError::Other(format!(
                "가격 파싱 실패: {} (price: {})",
                e, ticker.last_price
            ))
        })?;

        let mut state_map = state.write().await;
        let price_state = state_map
            .entry(symbol.to_string())
            .or_insert_with(PriceState::default);
        price_state.spot_price = Some(price);
        price_state.last_updated = Some(std::time::SystemTime::now());

        Ok(())
    }

    /// 선물 markPrice 메시지 처리
    async fn handle_futures_mark_price_message(
        text: &str,
        symbol: &str,
        state: Arc<TokioRwLock<HashMap<String, PriceState>>>,
    ) -> Result<(), ExchangeError> {
        #[derive(Debug, serde::Deserialize)]
        struct FuturesMarkPrice {
            #[serde(rename = "s")]
            symbol: String,
            #[serde(rename = "p")]
            mark_price: String,
        }

        let mark_price_data: FuturesMarkPrice = serde_json::from_str(text).map_err(|e| {
            ExchangeError::Other(format!("선물 markPrice 파싱 실패: {} (text: {})", e, text))
        })?;

        if mark_price_data.symbol != symbol {
            return Ok(());
        }

        let price: f64 = mark_price_data.mark_price.parse().map_err(|e| {
            ExchangeError::Other(format!(
                "가격 파싱 실패: {} (price: {})",
                e, mark_price_data.mark_price
            ))
        })?;

        let mut state_map = state.write().await;
        let price_state = state_map
            .entry(symbol.to_string())
            .or_insert_with(PriceState::default);
        price_state.futures_mark_price = Some(price);
        price_state.last_updated = Some(std::time::SystemTime::now());

        Ok(())
    }
}

