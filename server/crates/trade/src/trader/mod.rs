pub mod binance;
pub mod bithumb;

use async_trait::async_trait;
use interface::ExchangeError;

pub use binance::{BinanceTrader, OrderResponse};
pub use bithumb::BithumbTrader;

/// 프리미엄 거래소(spot)를 제어하기 위한 공통 인터페이스.
#[async_trait]
pub trait SpotExchangeTrader: Send + Sync {
    async fn ensure_exchange_info(&self) -> Result<(), ExchangeError>;
    async fn get_spot_price(&self, symbol: &str) -> Result<f64, ExchangeError>;
    fn clamp_spot_quantity(&self, symbol: &str, qty: f64) -> f64;
    async fn buy_spot(&self, symbol: &str, qty: f64) -> Result<OrderResponse, ExchangeError>;
    async fn sell_spot(&self, symbol: &str, qty: f64) -> Result<OrderResponse, ExchangeError>;
    async fn get_spot_balance(&self, asset: &str) -> Result<f64, ExchangeError>;
}

/// 헤지 거래소(선물)를 제어하기 위한 공통 인터페이스.
#[async_trait]
pub trait FuturesExchangeTrader: Send + Sync {
    async fn ensure_exchange_info(&self) -> Result<(), ExchangeError>;
    async fn ensure_account_setup(
        &self,
        symbol: &str,
        leverage: u32,
        isolated: bool,
    ) -> Result<(), ExchangeError>;
    async fn get_mark_price(&self, symbol: &str) -> Result<f64, ExchangeError>;
    fn clamp_futures_quantity(&self, symbol: &str, qty: f64) -> f64;
    async fn buy_futures(
        &self,
        symbol: &str,
        qty: f64,
        reduce_only: bool,
    ) -> Result<OrderResponse, ExchangeError>;
    async fn sell_futures(
        &self,
        symbol: &str,
        qty: f64,
        reduce_only: bool,
    ) -> Result<OrderResponse, ExchangeError>;
}
