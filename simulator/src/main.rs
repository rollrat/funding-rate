use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::time::{interval, Duration};
use axum::{Router, routing::get, routing::post, Extension};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

mod domain;
mod engine;
mod market;
mod gateway;
mod websocket;

use crate::engine::MatchingEngine;
use crate::market::{CompositeFlow, NoiseTrader, PassiveMM, SpikeGenerator, WhaleAgent, OrderFlowSource, RegimeState, Regime};
use crate::gateway::{get_orderbook, get_trades, post_order, OrderBookResponse, OrderJson};
use crate::websocket::{websocket_handler, create_broadcast, WebSocketMessage};

#[tokio::main]
async fn main() {
    // Initialize shared state
    let engine = Arc::new(RwLock::new(MatchingEngine::new()));

    // Set up market simulation sources
    let noise_trader = NoiseTrader;
    let passive_mm = PassiveMM::new(0.005); // e.g., 0.5% spread offset
    let spike_gen = SpikeGenerator::new(0.02, 50.0); // 기본 확률 2%, up to 50 quantity

    let sources: Vec<Box<dyn market::OrderFlowSource + Send>> = vec![
        Box::new(noise_trader),
        Box::new(passive_mm),
        Box::new(spike_gen),
    ];
    let mut composite_flow = CompositeFlow::new(sources);
    
    // WhaleAgent는 별도로 관리 (레짐 변경 시 리셋하기 위해)
    let mut whale_agent = WhaleAgent::new(crate::domain::OrderSide::Buy, 0.0); // 초기값, 나중에 리셋됨
    
    // RegimeState 초기화
    let mut regime = RegimeState::new();
    let mut prev_regime = regime.current;

    // Create broadcast channel for WebSocket
    let broadcast_tx = create_broadcast();
    let broadcast_tx_clone = broadcast_tx.clone();

    // Spawn the simulation loop in a background task
    let engine_clone = engine.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(50)); // 500ms로 변경 (요구사항에 따라)
        let mut rng = StdRng::from_entropy();
        loop {
            ticker.tick().await;
            
            // 1) 레짐 업데이트
            regime.step(&mut rng);
            
            // 레짐이 변경되었을 때 WhaleAgent 리셋
            if regime.current != prev_regime {
                match regime.current {
                    Regime::WhaleAccum => {
                        // WhaleAccum: Buy 방향으로 500~2000 사이 랜덤 목표 수량
                        let target = rng.gen_range(500.0..=2000.0);
                        whale_agent.reset(crate::domain::OrderSide::Buy, target);
                        eprintln!("[REGIME] Changed to WhaleAccum, target: {:.2}", target);
                    }
                    Regime::WhaleDump => {
                        // WhaleDump: Sell 방향으로 500~2000 사이 랜덤 목표 수량
                        let target = rng.gen_range(500.0..=2000.0);
                        whale_agent.reset(crate::domain::OrderSide::Sell, target);
                        eprintln!("[REGIME] Changed to WhaleDump, target: {:.2}", target);
                    }
                    _ => {
                        eprintln!("[REGIME] Changed to {:?}", regime.current);
                    }
                }
                prev_regime = regime.current;
            }
            
            // 2) 스냅샷 얻기
            let snapshot = {
                let eng = engine_clone.read().unwrap();
                eng.get_snapshot()
            };
            
            // 3) 모든 플로우에서 주문 생성 (레짐 전달)
            let mut orders: Vec<domain::Order> = composite_flow.generate(&snapshot, regime.current);
            
            // WhaleAgent 주문도 추가
            let whale_orders = whale_agent.generate(&snapshot, regime.current);
            orders.extend(whale_orders);
            
            if orders.is_empty() {
                continue; // skip if no orders generated this tick
            }
            
            // 4) 매칭 엔진에 주문 제출
            let mut eng = engine_clone.write().unwrap();
            let mut new_trades = Vec::new();
            for order in orders {
                // We ignore errors from engine here because our generators produce valid orders.
                // In a real scenario, we might log or handle EngineError.
                if let Ok(trades) = eng.submit_order(order) {
                    new_trades.extend(trades);
                }
            }
            
            // Broadcast updated orderbook via WebSocket
            // get_orderbook()은 내부 벡터 순서를 그대로 반환 (추가 정렬 없음)
            let (bids, asks) = eng.get_orderbook();
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
            
            let _ = broadcast_tx_clone.send(WebSocketMessage::OrderBook(orderbook));
            
            // 새로운 trades만 브로드캐스트 (있는 경우에만)
            if !new_trades.is_empty() {
                let _ = broadcast_tx_clone.send(WebSocketMessage::Trades(new_trades));
            }
        }
    });

    // Build the REST API router with our routes and shared state
    let app = Router::new()
        .route("/orderbook", get(get_orderbook))
        .route("/trades", get(get_trades))
        .route("/order", post(post_order))
        .route("/ws", get(websocket_handler))
        .layer(Extension(engine.clone())) // provide engine state to handlers
        .layer(Extension(broadcast_tx.clone())); // provide broadcast channel to handlers

    // Start HTTP server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running at http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await
        .expect("Failed to bind to address");
    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}

