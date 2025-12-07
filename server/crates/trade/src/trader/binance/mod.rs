//! Binance 거래소 트레이더 모듈
//!
//! 이 모듈은 Binance 거래소와의 상호작용을 담당합니다.
//! 기능별로 여러 하위 모듈로 분리되어 있습니다:
//! - `types`: 공통 타입 정의
//! - `order_client`: 주문 클라이언트 트레이트 및 HTTP 구현
//! - `spot_api`: Spot 거래 관련 API
//! - `futures_api`: Futures 거래 관련 API
//! - `price_feed`: 실시간 가격 피드 (WebSocket)
//! - `user_stream`: User Data Stream (WebSocket)
//! - `trader`: BinanceTrader 메인 구조체 및 트레이트 구현

pub mod futures_api;
pub mod order_client;
pub mod price_feed;
pub mod spot_api;
pub mod trader;
pub mod types;
pub mod user_stream;

// 공개 API
pub use futures_api::BinanceFuturesApi;
pub use order_client::{BinanceOrderClient, HttpBinanceOrderClient};
pub use price_feed::BinancePriceFeed;
pub use spot_api::BinanceSpotApi;
pub use trader::BinanceTrader;
pub use types::{
    clamp_quantity_with_filter, HedgedPair, LotSizeFilter, OrderResponse,
    PlaceFuturesOrderOptions, PlaceOrderOptions, PriceState,
};
pub use user_stream::{
    BalanceInfo, BalanceUpdate, ExecutionReport, OutboundAccountPosition, UserDataEvent,
};

