use crate::domain::{MarketSnapshot, Order};
use crate::market::{OrderFlowSource, Regime};

pub struct CompositeFlow {
    sources: Vec<Box<dyn OrderFlowSource + Send>>,
}

impl CompositeFlow {
    pub fn new(sources: Vec<Box<dyn OrderFlowSource + Send>>) -> Self {
        Self { sources }
    }
}

impl OrderFlowSource for CompositeFlow {
    fn generate(&mut self, snapshot: &MarketSnapshot, regime: Regime) -> Vec<Order> {
        let mut all_orders = Vec::new();
        for source in &mut self.sources {
            let mut orders = source.generate(snapshot, regime);
            all_orders.append(&mut orders);
        }
        all_orders
    }
}
