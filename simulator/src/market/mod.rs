pub mod composite;
pub mod noise_trader;
pub mod passive_mm;
pub mod regime;
pub mod spike_generator;
pub mod whale_agent;

pub use composite::CompositeFlow;
pub use noise_trader::NoiseTrader;
pub use passive_mm::PassiveMM;
pub use regime::{Regime, RegimeState};
pub use spike_generator::SpikeGenerator;
pub use whale_agent::WhaleAgent;

use crate::domain::{MarketSnapshot, Order};

pub trait OrderFlowSource {
    fn generate(&mut self, snapshot: &MarketSnapshot, regime: Regime) -> Vec<Order>;
}
