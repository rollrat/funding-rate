use chrono::Utc;
use uuid::Uuid;
use rand::Rng;
use crate::domain::{Order, OrderSide, OrderType, MarketSnapshot};
use crate::market::{OrderFlowSource, Regime};

pub struct WhaleAgent {
    side: OrderSide,      // Buy (매집) 또는 Sell (투매)
    target_position: f64, // 총 목표 수량
    filled: f64,          // 지금까지 누적 체결된 것으로 가정하고 사용
}

impl WhaleAgent {
    pub fn new(side: OrderSide, target_position: f64) -> Self {
        Self {
            side,
            target_position,
            filled: 0.0,
        }
    }

    pub fn reset(&mut self, side: OrderSide, target_position: f64) {
        self.side = side;
        self.target_position = target_position;
        self.filled = 0.0;
    }

    pub fn add_filled(&mut self, quantity: f64) {
        self.filled += quantity;
    }

    pub fn remaining(&self) -> f64 {
        (self.target_position - self.filled).max(0.0)
    }
}

impl OrderFlowSource for WhaleAgent {
    fn generate(&mut self, snapshot: &MarketSnapshot, regime: Regime) -> Vec<Order> {
        let mut orders = Vec::new();
        let mut rng = rand::thread_rng();

        // Regime이 WhaleAccum/WhaleDump일 때만 적극적으로 동작
        let is_active = match regime {
            Regime::WhaleAccum => self.side == OrderSide::Buy,
            Regime::WhaleDump => self.side == OrderSide::Sell,
            Regime::FlashPump => {
                // FlashPump에서도 한두 번 큰 Market order를 추가로 내서 가격을 세게 흔듦
                self.side == OrderSide::Buy && self.remaining() > 0.0
            }
            Regime::FlashCrash => {
                // FlashCrash에서도 한두 번 큰 Market order를 추가로 내서 가격을 세게 흔듦
                self.side == OrderSide::Sell && self.remaining() > 0.0
            }
            _ => false,
        };

        if !is_active {
            return orders;
        }

        let remaining = self.remaining();
        if remaining <= 0.0 {
            return orders;
        }

        // WhaleAccum/WhaleDump에서는 매 tick마다 남은 수량의 일부(3~10%)를 주문으로 제출
        // FlashPump/FlashCrash에서는 한두 번 큰 Market order를 추가로 내서 마지막에 한 번 더 가격을 세게 흔듦
        match regime {
            Regime::WhaleAccum | Regime::WhaleDump => {
                // 남은 수량의 3~10%를 주문
                let order_ratio = rng.gen_range(0.03..=0.10);
                let order_qty = (remaining * order_ratio).min(remaining);

                // 70%는 Market, 30%는 Limit으로 (조금 더 공격적인 스타일)
                let order_type = if rng.gen_bool(0.7) {
                    OrderType::Market
                } else {
                    OrderType::Limit
                };

                let price = match order_type {
                    OrderType::Limit => {
                        let base_price = snapshot.last_trade_price
                            .or(snapshot.best_bid)
                            .or(snapshot.best_ask)
                            .unwrap_or(100.0);
                        // Limit 주문의 경우, 매집이면 약간 높은 가격, 투매면 약간 낮은 가격으로
                        let offset = match self.side {
                            OrderSide::Buy => rng.gen_range(0.0..=0.01), // 매집: 약간 높은 가격
                            OrderSide::Sell => rng.gen_range(-0.01..=0.0), // 투매: 약간 낮은 가격
                        };
                        Some(base_price * (1.0 + offset))
                    }
                    OrderType::Market => None,
                };

                orders.push(Order {
                    id: Uuid::new_v4(),
                    side: self.side,
                    order_type,
                    price,
                    quantity: order_qty,
                    timestamp: Utc::now(),
                });

                // filled 업데이트는 실제로 체결된 후에 해야 하지만,
                // 여기서는 시뮬레이션 목적으로 주문 수량만큼 증가시킴
                // 실제로는 main.rs에서 체결 후 업데이트해야 함
            }
            Regime::FlashPump | Regime::FlashCrash => {
                // FlashPump/FlashCrash에서는 한두 번 큰 Market order를 추가로 내서 마지막에 한 번 더 가격을 세게 흔듦
                if rng.gen_bool(0.3) && remaining > 0.0 {
                    // 남은 수량의 20~50%를 한 번에 큰 Market order로
                    let order_ratio = rng.gen_range(0.2..=0.5);
                    let order_qty = (remaining * order_ratio).min(remaining);

                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: self.side,
                        order_type: OrderType::Market,
                        price: None,
                        quantity: order_qty,
                        timestamp: Utc::now(),
                    });
                }
            }
            _ => {}
        }

        orders
    }
}

