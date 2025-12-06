use crate::domain::{MarketSnapshot, Order, OrderSide, OrderType};
use crate::market::{OrderFlowSource, Regime};
use chrono::Utc;
use rand::Rng;
use uuid::Uuid;

pub struct NoiseTrader;

impl OrderFlowSource for NoiseTrader {
    fn generate(&mut self, snapshot: &MarketSnapshot, regime: Regime) -> Vec<Order> {
        let mut rng = rand::thread_rng();
        let mut orders = Vec::new();

        // 레짐에 따라 주문 개수와 수량 조정
        let (num_orders_range, qty_range, buy_bias) = match regime {
            Regime::Calm | Regime::Normal => {
                // Calm/Normal: tick당 주문 개수 0~2개, 수량 0.5~3
                ((0..=2), (0.5..=3.0), 0.5)
            }
            Regime::HighVol => {
                // HighVol: 주문 개수 1~4개, 수량 1~8로 확대
                ((1..=4), (1.0..=8.0), 0.5)
            }
            Regime::FlashCrash => {
                // FlashCrash: Sell 비중↑
                ((2..=5), (2.0..=10.0), 0.2) // 20% 확률로 Buy
            }
            Regime::FlashPump => {
                // FlashPump: Buy 비중↑
                ((2..=5), (2.0..=10.0), 0.8) // 80% 확률로 Buy
            }
            Regime::WhaleAccum | Regime::WhaleDump => {
                // WhaleAccum/WhaleDump: 전체 볼륨은 조금 늘리되, 방향성은 크게 바꾸지 않음
                ((1..=3), (1.0..=5.0), 0.5)
            }
        };

        let num_orders = rng.gen_range(num_orders_range);

        for _ in 0..num_orders {
            let side = if rng.gen_bool(buy_bias) {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };

            let order_type = if rng.gen_bool(0.7) {
                OrderType::Limit
            } else {
                OrderType::Market
            };

            let price = match order_type {
                OrderType::Limit => {
                    let base_price = snapshot
                        .last_trade_price
                        .or(snapshot.best_bid)
                        .or(snapshot.best_ask)
                        .unwrap_or(100.0);
                    // 레짐에 따라 가격 변동폭 조정
                    let offset_range = match regime {
                        Regime::Calm | Regime::Normal => -0.02..=0.02,
                        Regime::HighVol => -0.05..=0.05,
                        Regime::FlashCrash | Regime::FlashPump => -0.1..=0.1,
                        _ => -0.03..=0.03,
                    };
                    let offset = rng.gen_range(offset_range);
                    Some(base_price * (1.0 + offset))
                }
                OrderType::Market => None,
            };

            let quantity = rng.gen_range(qty_range.clone());

            orders.push(Order {
                id: Uuid::new_v4(),
                side,
                order_type,
                price,
                quantity,
                timestamp: Utc::now(),
            });
        }

        orders
    }
}
