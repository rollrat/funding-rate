use std::{sync::Arc, time::Duration};

use tokio::time::sleep;
use tracing::warn;

use crate::exchange::PerpExchange;
use crate::model::PerpSnapshot;
use crate::server::AppState;

pub fn start_collect_loop(
    exchanges: Vec<Arc<dyn PerpExchange>>,
    state: Arc<AppState>,
    interval: Duration,
) {
    tokio::spawn(async move {
        loop {
            let mut all: Vec<PerpSnapshot> = Vec::new();

            for ex in &exchanges {
                match ex.fetch_all().await {
                    Ok(mut v) => all.append(&mut v),
                    Err(e) => {
                        warn!("fetch error from {:?}: {:?}", ex.id(), e);
                    }
                }
            }

            // 정렬: OI 기준 내림차순
            all.sort_by(|a, b| {
                b.oi_usd
                    .partial_cmp(&a.oi_usd)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            {
                let mut guard = state.snapshots.write().await;
                *guard = all;
            }

            sleep(interval).await;
        }
    });
}
