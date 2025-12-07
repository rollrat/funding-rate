use async_trait::async_trait;
use std::sync::Arc;

use interface::ExchangeError;

use crate::trader::{FuturesExchangeTrader, SpotExchangeTrader};

use super::futures_api::BinanceFuturesApi;
use super::order_client::{BinanceOrderClient, HttpBinanceOrderClient};
use super::price_feed::BinancePriceFeed;
use super::spot_api::BinanceSpotApi;
use super::types::{HedgedPair, OrderResponse, PlaceFuturesOrderOptions, PlaceOrderOptions};
use super::user_stream::{BinanceUserStream, UserDataEvent};
use exchanges::BinanceClient;

pub struct BinanceTrader {
    pub order_client: Arc<dyn BinanceOrderClient>,
    pub spot: Arc<BinanceSpotApi>,
    pub futures: Arc<BinanceFuturesApi>,
    pub price_feed: Arc<BinancePriceFeed>,
    pub user_stream: Option<Arc<BinanceUserStream>>,
}

impl BinanceTrader {
    pub fn new() -> Result<Self, ExchangeError> {
        let spot_client = BinanceClient::with_credentials()
            .map_err(|e| ExchangeError::Other(format!("Failed to create spot client: {}", e)))?;
        let futures_client = BinanceClient::with_credentials()
            .map_err(|e| ExchangeError::Other(format!("Failed to create futures client: {}", e)))?;

        let order_client = Arc::new(HttpBinanceOrderClient::new(
            spot_client.clone(),
            futures_client.clone(),
        ));
        let spot = Arc::new(BinanceSpotApi::new(spot_client.clone()));
        let futures = Arc::new(BinanceFuturesApi::new(futures_client.clone()));
        let price_feed = Arc::new(BinancePriceFeed::new(
            spot_client.clone(),
            futures_client.clone(),
        ));
        let user_stream = Some(Arc::new(BinanceUserStream::new(spot_client)));

        Ok(Self {
            order_client,
            spot,
            futures,
            price_feed,
            user_stream,
        })
    }

    /// 거래소 이름 반환
    pub fn exchange_name(&self) -> &'static str {
        "binance"
    }

    /// 특정 심볼에 대한 WebSocket 리스너 시작
    /// 스팟 ticker와 선물 markPrice를 동시에 구독
    pub fn start_websocket_listener(&self, symbol: &str) {
        self.price_feed.start_symbol(symbol);
    }

    /// 스팟 현재가 조회 (메모리에서 읽기, 없으면 HTTP 폴백)
    pub async fn get_spot_price(&self, symbol: &str) -> Result<f64, ExchangeError> {
        self.price_feed.get_spot_price(symbol).await
    }

    /// 선물 마크 가격 조회 (메모리에서 읽기, 없으면 HTTP 폴백)
    pub async fn get_futures_mark_price(&self, symbol: &str) -> Result<f64, ExchangeError> {
        self.price_feed.get_futures_mark_price(symbol).await
    }

    /// 스팟 잔고 조회
    pub async fn get_spot_balance(&self, asset: &str) -> Result<f64, ExchangeError> {
        self.spot.get_balance(asset).await
    }

    /// 특정 심볼의 거래 수수료 조회
    pub async fn get_trade_fee_for_symbol(
        &self,
        symbol: &str,
    ) -> Result<interface::FeeInfo, ExchangeError> {
        self.spot.client().get_trade_fee_for_symbol(symbol).await
    }

    /// 선물 잔고 조회 (USDT 마진)
    pub async fn get_futures_balance(&self) -> Result<f64, ExchangeError> {
        self.futures.get_balance().await
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

    /// 스팟 수량을 거래소 규칙에 맞게 조정 (LOT_SIZE)
    /// exchangeInfo에서 가져온 실제 LOT_SIZE 필터를 사용
    pub fn clamp_spot_quantity(&self, symbol: &str, qty: f64) -> f64 {
        self.spot.clamp_quantity(symbol, qty)
    }

    /// 선물 수량을 거래소 규칙에 맞게 조정 (LOT_SIZE)
    /// exchangeInfo에서 가져온 실제 LOT_SIZE 필터를 사용
    pub fn clamp_futures_quantity(&self, symbol: &str, qty: f64) -> f64 {
        self.futures.clamp_quantity(symbol, qty)
    }

    /// target_net_qty 근처에서 스팟/선물 둘 다 LOT_SIZE를 만족하는 쌍을 찾는다.
    /// spot_fee_rate: 스팟 수수료율 (maker 또는 taker 중 선택)
    pub fn find_hedged_pair(
        &self,
        symbol: &str,
        target_net_qty: f64,
        spot_fee_rate: f64,
    ) -> Option<HedgedPair> {
        if target_net_qty <= 0.0 {
            return None;
        }

        // 선물 LOT_SIZE filter에서 stepSize를 가져와서 "한 스텝씩 줄여가며 탐색"에 사용
        let fut_lot = self.futures.get_lot_size(symbol)?;
        let fut_step = if fut_lot.step_size > 0.0 {
            fut_lot.step_size
        } else {
            // stepSize가 0이면 격자 정보가 없으니 그냥 한 번만 시도
            0.0
        };

        // 1) 먼저 target_net_qty를 기준으로 "선물 수량 후보"를 만든다.
        //    (선물 LOT_SIZE에 맞게 클램프)
        let mut fut_candidate = self.clamp_futures_quantity(symbol, target_net_qty);
        if fut_candidate <= 0.0 {
            return None;
        }

        // 허용 오차: 스팟/선물 스텝 중 더 작은 값의 절반 정도
        let spot_step = self
            .spot
            .get_lot_size(symbol)
            .map(|f| f.step_size)
            .unwrap_or(fut_step.max(1e-8)); // 그래도 0은 피하기

        let tol = spot_step.min(fut_step.max(spot_step)).abs() * 0.5;

        // 2) fut_candidate를 기준으로, 이에 맞는 스팟 주문 수량을 찾는다.
        //    안 맞으면 선물 수량을 한 step씩 줄여가며 재시도.
        let max_iters = 50;
        for _ in 0..max_iters {
            // 이 선물 수량을 "정확히" 덮고 싶다면, 스팟 순수량 == fut_candidate 여야 함.
            // spot_net = spot_order * (1 - fee) ⇒ spot_order = fut_candidate / (1 - fee)
            let ideal_spot_order = fut_candidate / (1.0 - spot_fee_rate);

            if !ideal_spot_order.is_finite() || ideal_spot_order <= 0.0 {
                break;
            }

            // 스팟 LOT_SIZE에 맞게 주문 수량 클램프
            let spot_order_qty = self.clamp_spot_quantity(symbol, ideal_spot_order);
            if spot_order_qty <= 0.0 {
                break;
            }

            // 클램프 후 "예상 스팟 순수량"
            let spot_net_qty_est = spot_order_qty * (1.0 - spot_fee_rate);

            // 이 조합에서의 예상 델타
            let delta = spot_net_qty_est - fut_candidate;

            // 델타가 허용 오차 내면 이 쌍을 채택
            if delta.abs() <= tol {
                return Some(HedgedPair {
                    spot_order_qty,
                    fut_order_qty: fut_candidate,
                    spot_net_qty_est,
                    delta_est: delta,
                });
            }

            // 더 안 맞으면 선물 수량을 한 step 줄여서 다시 시도
            if fut_step <= 0.0 {
                // step 정보가 없으면 더 이상 줄일 수 없음
                break;
            }

            let next_fut = fut_candidate - fut_step;
            let next_fut = self.clamp_futures_quantity(symbol, next_fut);
            if next_fut <= 0.0 || (next_fut - fut_candidate).abs() < 1e-12 {
                break;
            }
            fut_candidate = next_fut;
        }

        None
    }

    /// 스팟 exchangeInfo를 로드하여 LOT_SIZE 필터를 캐시에 저장
    pub async fn load_spot_exchange_info(&self) -> Result<(), ExchangeError> {
        self.spot.load_exchange_info().await
    }

    /// 선물 exchangeInfo를 로드하여 LOT_SIZE 필터를 캐시에 저장
    pub async fn load_futures_exchange_info(&self) -> Result<(), ExchangeError> {
        self.futures.load_exchange_info().await
    }

    /// 레거시 호환성을 위한 정적 메서드 (deprecated)
    /// 실제로는 clamp_spot_quantity 또는 clamp_futures_quantity를 사용해야 함
    #[deprecated(note = "Use clamp_spot_quantity or clamp_futures_quantity instead")]
    pub fn clamp_quantity(_symbol: &str, qty: f64) -> f64 {
        // 하위 호환성을 위해 간단한 구현 유지
        // 실제 사용 시에는 인스턴스 메서드를 사용해야 함
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
        self.order_client
            .place_spot_order(symbol, side, quantity, None, PlaceOrderOptions { test })
            .await
    }

    /// 선물 시장가 주문
    pub async fn place_futures_order(
        &self,
        symbol: &str,
        side: &str, // "BUY" or "SELL"
        quantity: f64,
        reduce_only: bool,
    ) -> Result<OrderResponse, ExchangeError> {
        self.order_client
            .place_futures_order(
                symbol,
                side,
                quantity,
                None,
                PlaceFuturesOrderOptions { reduce_only },
            )
            .await
    }

    /// User Data Stream 시작 및 이벤트 수신
    pub async fn start_user_data_stream<F>(&self, event_handler: F) -> Result<(), ExchangeError>
    where
        F: FnMut(UserDataEvent) + Send + 'static,
    {
        if let Some(user_stream) = &self.user_stream {
            user_stream.start(event_handler).await
        } else {
            Err(ExchangeError::Other(
                "User stream not initialized".to_string(),
            ))
        }
    }
}

#[async_trait]
impl SpotExchangeTrader for BinanceTrader {
    async fn ensure_exchange_info(&self) -> Result<(), ExchangeError> {
        self.load_spot_exchange_info().await
    }

    async fn get_spot_price(&self, symbol: &str) -> Result<f64, ExchangeError> {
        self.get_spot_price(symbol).await
    }

    fn clamp_spot_quantity(&self, symbol: &str, qty: f64) -> f64 {
        self.clamp_spot_quantity(symbol, qty)
    }

    async fn buy_spot(&self, symbol: &str, qty: f64) -> Result<OrderResponse, ExchangeError> {
        self.order_client
            .place_spot_order(symbol, "BUY", qty, None, PlaceOrderOptions { test: false })
            .await
    }

    async fn sell_spot(&self, symbol: &str, qty: f64) -> Result<OrderResponse, ExchangeError> {
        self.order_client
            .place_spot_order(symbol, "SELL", qty, None, PlaceOrderOptions { test: false })
            .await
    }

    async fn get_spot_balance(&self, asset: &str) -> Result<f64, ExchangeError> {
        self.get_spot_balance(asset).await
    }
}

#[async_trait]
impl FuturesExchangeTrader for BinanceTrader {
    async fn ensure_exchange_info(&self) -> Result<(), ExchangeError> {
        self.load_futures_exchange_info().await
    }

    async fn ensure_account_setup(
        &self,
        symbol: &str,
        leverage: u32,
        isolated: bool,
    ) -> Result<(), ExchangeError> {
        self.futures.ensure_setup(symbol, leverage, isolated).await
    }

    async fn get_mark_price(&self, symbol: &str) -> Result<f64, ExchangeError> {
        self.get_futures_mark_price(symbol).await
    }

    fn clamp_futures_quantity(&self, symbol: &str, qty: f64) -> f64 {
        self.clamp_futures_quantity(symbol, qty)
    }

    async fn buy_futures(
        &self,
        symbol: &str,
        qty: f64,
        reduce_only: bool,
    ) -> Result<OrderResponse, ExchangeError> {
        self.order_client
            .place_futures_order(
                symbol,
                "BUY",
                qty,
                None,
                PlaceFuturesOrderOptions { reduce_only },
            )
            .await
    }

    async fn sell_futures(
        &self,
        symbol: &str,
        qty: f64,
        reduce_only: bool,
    ) -> Result<OrderResponse, ExchangeError> {
        self.order_client
            .place_futures_order(
                symbol,
                "SELL",
                qty,
                None,
                PlaceFuturesOrderOptions { reduce_only },
            )
            .await
    }

}

