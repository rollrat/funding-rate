use serde::{Deserialize, Serialize};

/// 주문 응답
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub symbol: String,
    pub order_id: Option<u64>,
    pub client_order_id: Option<String>,
    pub executed_qty: Option<String>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// 주문 옵션 (Spot 주문용)
#[derive(Debug, Clone, Default)]
pub struct PlaceOrderOptions {
    pub test: bool,
}

/// 주문 옵션 (Futures 주문용)
#[derive(Debug, Clone, Default)]
pub struct PlaceFuturesOrderOptions {
    pub reduce_only: bool,
}

/// Binance LOT_SIZE 필터 정보
#[derive(Debug, Clone, Copy)]
pub struct LotSizeFilter {
    pub min_qty: f64,
    pub max_qty: f64,
    pub step_size: f64,
}

/// 실시간 가격 상태 (WebSocket에서 업데이트)
#[derive(Debug, Clone, Default)]
pub struct PriceState {
    pub spot_price: Option<f64>,
    pub futures_mark_price: Option<f64>,
    pub last_updated: Option<std::time::SystemTime>,
}

/// 헤지된 주문 쌍
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct HedgedPair {
    /// 스팟 주문에 실제로 넣을 수량 (LOT_SIZE 만족)
    pub spot_order_qty: f64,
    /// 선물 주문에 실제로 넣을 수량 (LOT_SIZE 만족)
    pub fut_order_qty: f64,
    /// 수수료 반영 후 예상 스팟 순수량
    pub spot_net_qty_est: f64,
    /// 예상 잔여 델타 (spot_net - fut)
    pub delta_est: f64,
}

/// LOT_SIZE 필터를 사용하여 수량을 clamp하는 헬퍼 함수
pub fn clamp_quantity_with_filter(filter: LotSizeFilter, qty: f64) -> f64 {
    const BASE_PRECISION: u32 = 8;

    if qty <= 0.0 {
        return 0.0;
    }

    // 1) precision 잘라내기 (floor)
    let pow = 10f64.powi(BASE_PRECISION as i32);
    let mut qty = (qty * pow).floor() / pow;

    // 2) stepSize 처리
    if filter.step_size > 0.0 {
        let steps = (qty / filter.step_size).floor();
        qty = steps * filter.step_size;
    }

    // 3) minQty 미만이면 invalid → 0이 아니라 "그냥 에러"로 처리해야 맞음
    if qty < filter.min_qty {
        return 0.0; // ← but ideally, return Err(...)
    }

    // 4) maxQty clamp
    if qty > filter.max_qty {
        qty = filter.max_qty;
    }

    qty
}
