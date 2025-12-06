use crate::domain::{MarketSnapshot, Order, OrderSide, OrderType, Trade};
use chrono::Utc;
use std::collections::VecDeque;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Limit order must have a price specified")]
    PriceMissing,
    #[error("Order quantity must be positive")]
    InvalidQuantity,
}

pub struct MatchingEngine {
    bids: Vec<Order>,         // sorted by price desc
    asks: Vec<Order>,         // sorted by price asc
    trades: VecDeque<Trade>,  // recent trades (queue for FIFO removal)
    max_trade_history: usize, // max number of stored trades
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
            trades: VecDeque::new(),
            max_trade_history: 100, // keep up to 100 recent trades
        }
    }

    /// Returns a snapshot of current market (best bid/ask and last trade price).
    /// best_bid는 bids[0]의 가격, best_ask는 asks[0]의 가격을 사용합니다.
    pub fn get_snapshot(&self) -> MarketSnapshot {
        let best_bid = self.bids.first().and_then(|o| o.price);
        let best_ask = self.asks.first().and_then(|o| o.price);
        let last_trade_price = self.trades.back().map(|t| t.price);

        // 디버깅: best_bid/best_ask가 실제 오더북 최상단과 일치하는지 확인
        #[cfg(debug_assertions)]
        {
            if let Some(bb) = best_bid {
                eprintln!(
                    "[DEBUG] Snapshot best_bid: {:.2}, bids[0] price: {:.2}",
                    bb,
                    self.bids.first().and_then(|o| o.price).unwrap_or(0.0)
                );
            }
            if let Some(ba) = best_ask {
                eprintln!(
                    "[DEBUG] Snapshot best_ask: {:.2}, asks[0] price: {:.2}",
                    ba,
                    self.asks.first().and_then(|o| o.price).unwrap_or(0.0)
                );
            }
        }

        MarketSnapshot {
            best_bid,
            best_ask,
            last_trade_price,
        }
    }

    /// Submits an order and returns a list of trades that occurred.
    pub fn submit_order(&mut self, mut order: Order) -> Result<Vec<Trade>, EngineError> {
        // Validate order
        match order.order_type {
            OrderType::Limit => {
                if order.price.is_none() {
                    return Err(EngineError::PriceMissing);
                }
            }
            OrderType::Market => {
                // Market orders don't need price validation
            }
        }

        if order.quantity <= 0.0 {
            return Err(EngineError::InvalidQuantity);
        }

        let mut trades = Vec::new();
        let remaining_qty = match order.side {
            OrderSide::Buy => self.match_buy_order(&mut order, &mut trades)?,
            OrderSide::Sell => self.match_sell_order(&mut order, &mut trades)?,
        };

        // If there's remaining quantity and it's a limit order, add to book
        if remaining_qty > 0.0 && matches!(order.order_type, OrderType::Limit) {
            order.quantity = remaining_qty;
            match order.side {
                OrderSide::Buy => {
                    self.insert_bid(order);
                }
                OrderSide::Sell => {
                    self.insert_ask(order);
                }
            }
        }

        Ok(trades)
    }

    fn match_buy_order(
        &mut self,
        order: &mut Order,
        trades: &mut Vec<Trade>,
    ) -> Result<f64, EngineError> {
        let mut remaining_qty = order.quantity;
        let buy_price = order.price;

        while remaining_qty > 0.0 && !self.asks.is_empty() {
            let ask = &self.asks[0];
            let ask_price = ask.price.unwrap_or(0.0);

            // Check if we can match
            let can_match = match buy_price {
                Some(price) => ask_price <= price, // Limit order: only match if ask price <= buy price
                None => true,                      // Market order: match at any price
            };

            if !can_match {
                break;
            }

            let trade_qty = remaining_qty.min(ask.quantity);
            let trade_price = ask_price; // Price-time priority: use resting order's price

            // Create trade (매수 주문이 체결됨)
            let trade = Trade {
                price: trade_price,
                quantity: trade_qty,
                side: OrderSide::Buy, // 매수 주문이 체결됨
                timestamp: Utc::now(),
            };
            trades.push(trade.clone());
            self.trades.push_back(trade);

            // Update quantities
            remaining_qty -= trade_qty;
            let mut matched_ask = self.asks.remove(0);

            if matched_ask.quantity > trade_qty {
                // Partial fill of resting order
                matched_ask.quantity -= trade_qty;
                self.insert_ask(matched_ask);
            }
        }

        // Trim trade history if needed
        while self.trades.len() > self.max_trade_history {
            self.trades.pop_front();
        }

        Ok(remaining_qty)
    }

    fn match_sell_order(
        &mut self,
        order: &mut Order,
        trades: &mut Vec<Trade>,
    ) -> Result<f64, EngineError> {
        let mut remaining_qty = order.quantity;
        let sell_price = order.price;

        while remaining_qty > 0.0 && !self.bids.is_empty() {
            let bid = &self.bids[0];
            let bid_price = bid.price.unwrap_or(0.0);

            // Check if we can match
            let can_match = match sell_price {
                Some(price) => bid_price >= price, // Limit order: only match if bid price >= sell price
                None => true,                      // Market order: match at any price
            };

            if !can_match {
                break;
            }

            let trade_qty = remaining_qty.min(bid.quantity);
            let trade_price = bid_price; // Price-time priority: use resting order's price

            // Create trade (매도 주문이 체결됨)
            let trade = Trade {
                price: trade_price,
                quantity: trade_qty,
                side: OrderSide::Sell, // 매도 주문이 체결됨
                timestamp: Utc::now(),
            };
            trades.push(trade.clone());
            self.trades.push_back(trade);

            // Update quantities
            remaining_qty -= trade_qty;
            let mut matched_bid = self.bids.remove(0);

            if matched_bid.quantity > trade_qty {
                // Partial fill of resting order
                matched_bid.quantity -= trade_qty;
                self.insert_bid(matched_bid);
            }
        }

        // Trim trade history if needed
        while self.trades.len() > self.max_trade_history {
            self.trades.pop_front();
        }

        Ok(remaining_qty)
    }

    fn insert_bid(&mut self, order: Order) {
        // Insert in descending price order (highest first = index 0)
        // bids는 가격 내림차순이므로, 높은 가격이 앞에 와야 함
        let price = order.price.unwrap_or(0.0);
        let pos = self
            .bids
            .binary_search_by(|o| {
                // 내림차순 정렬: 높은 가격이 앞에 오도록
                // 비교: o.price와 price를 비교할 때, price가 더 크면 앞에 와야 함
                o.price
                    .unwrap_or(0.0)
                    .partial_cmp(&price)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .reverse() // reverse를 해서 내림차순으로 만듦
            })
            .unwrap_or_else(|e| e);
        self.bids.insert(pos, order);
    }

    fn insert_ask(&mut self, order: Order) {
        // Insert in ascending price order (lowest first = index 0)
        // asks는 가격 오름차순이므로, 낮은 가격이 앞에 와야 함
        let price = order.price.unwrap_or(0.0);
        let pos = self
            .asks
            .binary_search_by(|o| {
                // 오름차순 정렬: 낮은 가격이 앞에 오도록
                o.price
                    .unwrap_or(0.0)
                    .partial_cmp(&price)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or_else(|e| e);
        self.asks.insert(pos, order);
    }

    /// Returns orderbook as-is: bids는 가격 내림차순(index 0이 최고가), asks는 가격 오름차순(index 0이 최저가)
    /// 추가 정렬 없이 내부 벡터 순서를 그대로 반환합니다.
    pub fn get_orderbook(&self) -> (Vec<&Order>, Vec<&Order>) {
        (self.bids.iter().collect(), self.asks.iter().collect())
    }

    pub fn get_trades(&self) -> Vec<&Trade> {
        self.trades.iter().collect()
    }
}
