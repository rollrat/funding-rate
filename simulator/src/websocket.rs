use axum::{
    extract::{
        ws::{Message, WebSocket},
        Extension, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde_json;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

use crate::domain::Trade;
use crate::engine::MatchingEngine;
use crate::gateway::{OrderBookResponse, OrderJson};

pub type BroadcastTx = broadcast::Sender<WebSocketMessage>;

#[derive(Debug, Clone, serde::Serialize)]
pub enum WebSocketMessage {
    OrderBook(OrderBookResponse),
    Trades(Vec<Trade>), // 새로운 trades만 포함 (전체가 아님)
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(engine): Extension<Arc<RwLock<MatchingEngine>>>,
    Extension(tx): Extension<BroadcastTx>,
) -> Response {
    // Send initial data before upgrading
    // get_orderbook()은 내부 벡터 순서를 그대로 반환 (추가 정렬 없음)
    let initial_orderbook = {
        let engine = engine.read().unwrap();
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
        OrderBookResponse {
            bids: bids_json,
            asks: asks_json,
        }
    };

    let initial_trades = {
        let engine = engine.read().unwrap();
        engine.get_trades().into_iter().cloned().collect::<Vec<Trade>>()
    };

    // Send initial messages
    let _ = tx.send(WebSocketMessage::OrderBook(initial_orderbook));
    let _ = tx.send(WebSocketMessage::Trades(initial_trades));

    ws.on_upgrade(|socket| handle_socket(socket, tx))
}

async fn handle_socket(socket: WebSocket, tx: BroadcastTx) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = tx.subscribe();

    // Spawn task to forward broadcast messages to websocket
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Spawn task to receive messages from websocket (for ping/pong)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

pub fn create_broadcast() -> BroadcastTx {
    broadcast::channel(100).0
}

