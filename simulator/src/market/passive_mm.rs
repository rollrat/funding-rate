use chrono::Utc;
use uuid::Uuid;
use rand::Rng;
use crate::domain::{Order, OrderSide, OrderType, MarketSnapshot};
use crate::market::{OrderFlowSource, Regime};

pub struct PassiveMM {
    spread_offset: f64, // e.g., 0.005 = 0.5% spread
}

impl PassiveMM {
    pub fn new(spread_offset: f64) -> Self {
        Self { spread_offset }
    }
}

impl OrderFlowSource for PassiveMM {
    fn generate(&mut self, snapshot: &MarketSnapshot, regime: Regime) -> Vec<Order> {
        let mut orders = Vec::new();
        let mut rng = rand::thread_rng();

        // Only generate if we have a reference price
        let mid_price = match (snapshot.best_bid, snapshot.best_ask) {
            (Some(bid), Some(ask)) => (bid + ask) / 2.0,
            (Some(bid), None) | (None, Some(bid)) => bid,
            (None, None) => {
                snapshot.last_trade_price.unwrap_or(100.0)
            }
        };

        // 레짐에 따라 스프레드와 유동성 구조 조정
        match regime {
            Regime::Calm | Regime::Normal => {
                // Calm/Normal: tight spread로 양쪽에 유동성 제공
                let spread = self.spread_offset;
                if rng.gen_bool(0.6) {
                    let bid_price = mid_price * (1.0 - spread);
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: OrderSide::Buy,
                        order_type: OrderType::Limit,
                        price: Some(bid_price),
                        quantity: rng.gen_range(5.0..=15.0),
                        timestamp: Utc::now(),
                    });
                }
                if rng.gen_bool(0.6) {
                    let ask_price = mid_price * (1.0 + spread);
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: OrderSide::Sell,
                        order_type: OrderType::Limit,
                        price: Some(ask_price),
                        quantity: rng.gen_range(5.0..=15.0),
                        timestamp: Utc::now(),
                    });
                }
            }
            Regime::HighVol => {
                // HighVol: 스프레드를 넓히고, 한쪽(가격이 밀리는 방향)의 수량을 줄여서 슬리피지가 커지게
                let spread = self.spread_offset * 2.0; // 스프레드 2배
                let price_direction = if snapshot.last_trade_price.is_some() {
                    let last = snapshot.last_trade_price.unwrap();
                    let mid = mid_price;
                    if last < mid {
                        // 가격이 하락 중이면 bid 쪽 수량 줄임
                        -1.0
                    } else {
                        // 가격이 상승 중이면 ask 쪽 수량 줄임
                        1.0
                    }
                } else {
                    0.0
                };

                if rng.gen_bool(0.5) {
                    let bid_price = mid_price * (1.0 - spread);
                    let qty = if price_direction < 0.0 {
                        rng.gen_range(2.0..=8.0) // bid 쪽 수량 줄임
                    } else {
                        rng.gen_range(5.0..=15.0)
                    };
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: OrderSide::Buy,
                        order_type: OrderType::Limit,
                        price: Some(bid_price),
                        quantity: qty,
                        timestamp: Utc::now(),
                    });
                }
                if rng.gen_bool(0.5) {
                    let ask_price = mid_price * (1.0 + spread);
                    let qty = if price_direction > 0.0 {
                        rng.gen_range(2.0..=8.0) // ask 쪽 수량 줄임
                    } else {
                        rng.gen_range(5.0..=15.0)
                    };
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: OrderSide::Sell,
                        order_type: OrderType::Limit,
                        price: Some(ask_price),
                        quantity: qty,
                        timestamp: Utc::now(),
                    });
                }
            }
            Regime::FlashCrash => {
                // FlashCrash: ask 쪽 유동성 거의 없고, bid 쪽은 조금씩 쌓이는 느낌
                if rng.gen_bool(0.8) {
                    let bid_price = mid_price * (1.0 - self.spread_offset * 1.5);
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: OrderSide::Buy,
                        order_type: OrderType::Limit,
                        price: Some(bid_price),
                        quantity: rng.gen_range(3.0..=10.0),
                        timestamp: Utc::now(),
                    });
                }
                if rng.gen_bool(0.1) {
                    // ask 쪽은 거의 안 넣음
                    let ask_price = mid_price * (1.0 + self.spread_offset * 3.0);
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: OrderSide::Sell,
                        order_type: OrderType::Limit,
                        price: Some(ask_price),
                        quantity: rng.gen_range(1.0..=3.0),
                        timestamp: Utc::now(),
                    });
                }
            }
            Regime::FlashPump => {
                // FlashPump: 반대로 bid 쪽이 얇고 ask 쪽에만 두껍게 쌓이는 식
                if rng.gen_bool(0.1) {
                    // bid 쪽은 거의 안 넣음
                    let bid_price = mid_price * (1.0 - self.spread_offset * 3.0);
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: OrderSide::Buy,
                        order_type: OrderType::Limit,
                        price: Some(bid_price),
                        quantity: rng.gen_range(1.0..=3.0),
                        timestamp: Utc::now(),
                    });
                }
                if rng.gen_bool(0.8) {
                    let ask_price = mid_price * (1.0 + self.spread_offset * 1.5);
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: OrderSide::Sell,
                        order_type: OrderType::Limit,
                        price: Some(ask_price),
                        quantity: rng.gen_range(3.0..=10.0),
                        timestamp: Utc::now(),
                    });
                }
            }
            Regime::WhaleAccum | Regime::WhaleDump => {
                // WhaleAccum/WhaleDump: 일반적인 유동성 제공
                if rng.gen_bool(0.5) {
                    let bid_price = mid_price * (1.0 - self.spread_offset);
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: OrderSide::Buy,
                        order_type: OrderType::Limit,
                        price: Some(bid_price),
                        quantity: rng.gen_range(5.0..=15.0),
                        timestamp: Utc::now(),
                    });
                }
                if rng.gen_bool(0.5) {
                    let ask_price = mid_price * (1.0 + self.spread_offset);
                    orders.push(Order {
                        id: Uuid::new_v4(),
                        side: OrderSide::Sell,
                        order_type: OrderType::Limit,
                        price: Some(ask_price),
                        quantity: rng.gen_range(5.0..=15.0),
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        orders
    }
}

