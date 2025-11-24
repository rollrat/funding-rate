pub mod state;
pub mod strategy;

pub use crate::trader::{binance::BinanceTrader, bithumb::BithumbTrader};
pub use state::ArbitrageState;
pub use strategy::{
    cross_basis::CrossBasisArbitrageStrategy, intra_basis::IntraBasisArbitrageStrategy,
    StrategyParams,
};
