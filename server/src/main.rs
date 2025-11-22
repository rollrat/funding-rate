use std::{sync::Arc, time::Duration};

use tracing_subscriber::{fmt, EnvFilter};

mod collector;
mod exchange;
mod model;
mod server;

use crate::exchange::{BinanceClient, BybitClient, OkxClient, PerpExchange};
use crate::server::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // init logging
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let state = Arc::new(AppState::new());

    // set up exchanges
    let binance: Arc<dyn PerpExchange> = Arc::new(BinanceClient::new());
    let bybit: Arc<dyn PerpExchange> = Arc::new(BybitClient::new());
    let okx: Arc<dyn PerpExchange> = Arc::new(OkxClient::new());

    let exchanges = vec![binance, bybit, okx];

    // start background collector
    collector::start_collect_loop(exchanges, state.clone(), Duration::from_secs(10));

    // start HTTP server on 8080
    server::serve(state, 12090).await?;

    Ok(())
}
