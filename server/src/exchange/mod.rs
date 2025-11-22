use async_trait::async_trait;
use thiserror::Error;

use crate::model::{ExchangeId, PerpSnapshot};

pub mod binance;
pub mod bybit;
pub mod okx;

#[derive(Error, Debug)]
pub enum ExchangeError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("other error: {0}")]
    Other(String),
}

#[async_trait]
pub trait PerpExchange: Send + Sync {
    fn id(&self) -> ExchangeId;

    async fn fetch_all(&self) -> Result<Vec<PerpSnapshot>, ExchangeError>;
}

// Convenience re-exports
pub use binance::BinanceClient;
pub use bybit::BybitClient;
pub use okx::OkxClient;
