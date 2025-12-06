use axum::{
    extract::Extension,
    http::StatusCode,
    response::Json,
};
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::domain::{Order, OrderSide, OrderType};
use crate::engine::MatchingEngine;
use crate::websocket::{BroadcastTx, WebSocketMessage};

#[derive(Debug, Deserialize)]
pub struct OrderRequest {
    pub side: String,
    pub order_type: String,
    pub price: Option<f64>,
    pub quantity: f64,
}

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: Uuid,
    pub status: String,
    pub trades: Vec<crate::domain::Trade>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderBookResponse {
    pub bids: Vec<OrderJson>,
    pub asks: Vec<OrderJson>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderJson {
    pub id: Uuid,
    pub side: String,
    pub order_type: String,
    pub price: Option<f64>,
    pub quantity: f64,
    pub timestamp: String,
}

pub async fn get_orderbook(
    Extension(engine): Extension<Arc<RwLock<MatchingEngine>>>,
) -> Json<OrderBookResponse> {
    let engine = engine.read().unwrap();
    // get_orderbook()은 내부 벡터 순서를 그대로 반환:
    // bids는 가격 내림차순 (index 0이 최고가), asks는 가격 오름차순 (index 0이 최저가)
    let (bids, asks) = engine.get_orderbook();

    // 추가 정렬 없이 그대로 반환 (bids는 index 0부터, asks도 index 0부터)
    let bids_json: Vec<OrderJson> = bids
        .iter()
        .map(|o| OrderJson {
            id: o.id,
            side: format!("{:?}", o.side),
            order_type: format!("{:?}", o.order_type),
            price: o.price,
            quantity: o.quantity,
            timestamp: o.timestamp.to_rfc3339(),
        })
        .collect();

    let asks_json: Vec<OrderJson> = asks
        .iter()
        .map(|o| OrderJson {
            id: o.id,
            side: format!("{:?}", o.side),
            order_type: format!("{:?}", o.order_type),
            price: o.price,
            quantity: o.quantity,
            timestamp: o.timestamp.to_rfc3339(),
        })
        .collect();

    Json(OrderBookResponse {
        bids: bids_json,
        asks: asks_json,
    })
}

pub async fn get_trades(
    Extension(engine): Extension<Arc<RwLock<MatchingEngine>>>,
) -> Json<Vec<crate::domain::Trade>> {
    let engine = engine.read().unwrap();
    let trades: Vec<crate::domain::Trade> = engine.get_trades().into_iter().cloned().collect();
    Json(trades)
}

pub async fn post_order(
    Extension(engine): Extension<Arc<RwLock<MatchingEngine>>>,
    Extension(broadcast_tx): Extension<BroadcastTx>,
    Json(req): Json<OrderRequest>,
) -> Result<Json<OrderResponse>, StatusCode> {
    // Parse side
    let side = match req.side.as_str() {
        "Buy" => OrderSide::Buy,
        "Sell" => OrderSide::Sell,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Parse order type
    let order_type = match req.order_type.as_str() {
        "Limit" => OrderType::Limit,
        "Market" => OrderType::Market,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Validate price for limit orders
    let price = match order_type {
        OrderType::Limit => {
            Some(req.price.ok_or(StatusCode::BAD_REQUEST)?)
        }
        OrderType::Market => None,
    };

    // Create order
    let new_order = Order {
        id: Uuid::new_v4(),
        side,
        order_type,
        price,
        quantity: req.quantity,
        timestamp: Utc::now(),
    };

    // Submit to engine
    let mut engine = engine.write().unwrap();
    match engine.submit_order(new_order.clone()) {
        Ok(trades) => {
            let status = if trades.is_empty() {
                if matches!(order_type, OrderType::Market) {
                    "NotFilled"
                } else {
                    "Open"
                }
            } else {
                // Check if order is fully filled
                let total_filled: f64 = trades.iter().map(|t| t.quantity).sum();
                if total_filled >= new_order.quantity {
                    "Filled"
                } else {
                    "PartiallyFilled"
                }
            };

            // Get order ID from book if still open, otherwise use new order ID
            let order_id = if status == "Open" || status == "PartiallyFilled" {
                let (bids, asks) = engine.get_orderbook();
                match side {
                    OrderSide::Buy => bids.first().map(|o| o.id).unwrap_or(new_order.id),
                    OrderSide::Sell => asks.first().map(|o| o.id).unwrap_or(new_order.id),
                }
            } else {
                new_order.id
            };

            // Broadcast updated orderbook and trades via WebSocket
            // get_orderbook()은 내부 벡터 순서를 그대로 반환 (추가 정렬 없음)
            let (bids, asks) = engine.get_orderbook();
            let bids_json: Vec<OrderJson> = bids
                .iter()
                .map(|o| OrderJson {
                    id: o.id,
                    side: format!("{:?}", o.side),
                    order_type: format!("{:?}", o.order_type),
                    price: o.price,
                    quantity: o.quantity,
                    timestamp: o.timestamp.to_rfc3339(),
                })
                .collect();
            let asks_json: Vec<OrderJson> = asks
                .iter()
                .map(|o| OrderJson {
                    id: o.id,
                    side: format!("{:?}", o.side),
                    order_type: format!("{:?}", o.order_type),
                    price: o.price,
                    quantity: o.quantity,
                    timestamp: o.timestamp.to_rfc3339(),
                })
                .collect();
            
            let orderbook = OrderBookResponse {
                bids: bids_json,
                asks: asks_json,
            };
            
            let _ = broadcast_tx.send(WebSocketMessage::OrderBook(orderbook));
            
            // 새로운 trades만 브로드캐스트 (있는 경우에만)
            if !trades.is_empty() {
                let new_trades: Vec<crate::domain::Trade> = trades.iter().cloned().collect();
                let _ = broadcast_tx.send(WebSocketMessage::Trades(new_trades));
            }

            Ok(Json(OrderResponse {
                id: order_id,
                status: status.to_string(),
                trades,
            }))
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

