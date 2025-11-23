use std::{sync::Arc, time::Duration};

use color_eyre::eyre;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use exchanges::{
    bithumb::BithumbClient, BinanceClient, BitgetClient, BybitClient, OkxClient, PerpExchange,
    SpotExchange,
};
use oracle::server::AppState;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // init error reporting
    color_eyre::install()?;

    // init logging
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    info!("서버 시작 중...");

    let state = Arc::new(AppState::new());

    // set up perp exchanges
    let perp_exchanges: Vec<Arc<dyn PerpExchange>> = vec![
        Arc::new(BinanceClient::new()),
        Arc::new(BybitClient::new()),
        Arc::new(OkxClient::new()),
        Arc::new(BitgetClient::new()),
    ];

    // set up spot exchanges
    let spot_exchanges: Vec<Arc<dyn SpotExchange>> = vec![
        Arc::new(BinanceClient::new()),
        Arc::new(BybitClient::new()),
        Arc::new(OkxClient::new()),
        Arc::new(BitgetClient::new()),
        Arc::new(BithumbClient::new()),
    ];

    // start background collector
    oracle::collector::start_collect_loop(
        perp_exchanges,
        spot_exchanges,
        state.clone(),
        Duration::from_secs(10),
    );

    // start HTTP server on 8080
    oracle::server::serve(state, 12090).await?;

    Ok(())
}
