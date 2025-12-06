use chrono::Utc;
use uuid::Uuid;
use rand::Rng;
use crate::domain::{Order, OrderSide, OrderType, MarketSnapshot};
use crate::market::{OrderFlowSource, Regime};

pub struct SpikeGenerator {
    base_probability: f64, // 기본 확률
    max_quantity: f64,
}

impl SpikeGenerator {
    pub fn new(probability: f64, max_quantity: f64) -> Self {
        Self {
            base_probability: probability,
            max_quantity,
        }
    }
}

impl OrderFlowSource for SpikeGenerator {
    fn generate(&mut self, _snapshot: &MarketSnapshot, regime: Regime) -> Vec<Order> {
        let mut rng = rand::thread_rng();
        let mut orders = Vec::new();

        // Regime에 따라 probability 조정
        let probability = match regime {
            Regime::Calm | Regime::Normal => 0.005, // 거의 안 터지도록
            Regime::HighVol => 0.05,
            Regime::FlashCrash | Regime::FlashPump => 0.2, // 크게
            Regime::WhaleAccum | Regime::WhaleDump => 0.01,
        };

        if rng.gen_bool(probability) {
            // FlashCrash일 때는 Sell 방향 market order 위주,
            // FlashPump일 때는 Buy 방향 market order 위주
            let side = match regime {
                Regime::FlashCrash => {
                    // 80% 확률로 Sell
                    if rng.gen_bool(0.8) {
                        OrderSide::Sell
                    } else {
                        OrderSide::Buy
                    }
                }
                Regime::FlashPump => {
                    // 80% 확률로 Buy
                    if rng.gen_bool(0.8) {
                        OrderSide::Buy
                    } else {
                        OrderSide::Sell
                    }
                }
                _ => {
                    // 그 외에는 랜덤
                    if rng.gen_bool(0.5) {
                        OrderSide::Buy
                    } else {
                        OrderSide::Sell
                    }
                }
            };

            // Large market order (price not needed for market orders)
            let qty_multiplier = match regime {
                Regime::FlashCrash | Regime::FlashPump => 1.5, // 더 큰 수량
                Regime::HighVol => 1.2,
                _ => 1.0,
            };
            orders.push(Order {
                id: Uuid::new_v4(),
                side,
                order_type: OrderType::Market,
                price: None,
                quantity: rng.gen_range(self.max_quantity * 0.5 * qty_multiplier..=self.max_quantity * qty_multiplier),
                timestamp: Utc::now(),
            });
        }

        orders
    }
}

