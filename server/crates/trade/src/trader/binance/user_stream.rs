use std::collections::BTreeMap;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

use exchanges::binance::{generate_signature, get_timestamp};
use exchanges::BinanceClient;
use interface::ExchangeError;

const WS_API_URL: &str = "wss://ws-api.binance.com/ws-api/v3";

/// Binance User Stream: User Data Stream WebSocket 관리
pub struct BinanceUserStream {
    spot_client: BinanceClient,
}

impl BinanceUserStream {
    pub fn new(spot_client: BinanceClient) -> Self {
        Self { spot_client }
    }

    /// User Data Stream 시작 및 이벤트 수신
    pub async fn start<F>(&self, mut event_handler: F) -> Result<(), ExchangeError>
    where
        F: FnMut(UserDataEvent) + Send + 'static,
    {
        loop {
            match self.connect(&mut event_handler).await {
                Ok(_) => {
                    warn!("User Data Stream WebSocket 연결이 종료되었습니다. 재연결 시도...");
                }
                Err(e) => {
                    error!("User Data Stream WebSocket 오류: {:?}. 재연결 시도...", e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    /// WebSocket 연결 및 메시지 수신
    async fn connect<F>(&self, event_handler: &mut F) -> Result<(), ExchangeError>
    where
        F: FnMut(UserDataEvent) + Send + 'static,
    {
        let api_key = self
            .spot_client
            .api_key
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API key not set".to_string()))?;
        let api_secret = self
            .spot_client
            .api_secret
            .as_ref()
            .ok_or_else(|| ExchangeError::Other("API secret not set".to_string()))?;

        let (ws_stream, _) = connect_async(WS_API_URL)
            .await
            .map_err(|e| ExchangeError::Other(format!("WebSocket 연결 실패: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();

        info!("User Data Stream WebSocket 연결 성공: {}", WS_API_URL);

        // 구독 요청 전송
        let _request_id = Self::subscribe_user_data_stream(&mut write, api_key, api_secret).await?;

        // 구독 응답 대기
        if let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("Subscribe response: {}", text);
                    let response: WsResponse = serde_json::from_str(&text)
                        .map_err(|e| ExchangeError::Other(format!("응답 파싱 실패: {}", e)))?;

                    if let Some(error) = response.error {
                        return Err(ExchangeError::Other(format!(
                            "구독 실패: code={:?}, msg={:?}",
                            error.code, error.msg
                        )));
                    }

                    if let Some(result) = response.result {
                        info!(
                            "구독 성공: subscriptionId={:?}",
                            result.get("subscriptionId")
                        );
                    }
                }
                Ok(Message::Close(_)) => {
                    warn!("WebSocket 연결이 닫혔습니다");
                    return Ok(());
                }
                Err(e) => {
                    return Err(ExchangeError::Other(format!(
                        "구독 응답 수신 오류: {:?}",
                        e
                    )));
                }
                _ => {}
            }
        }

        info!("User Data Stream 이벤트 수신 대기 중...");

        // 이벤트 수신 루프
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = Self::handle_user_data_message(&text, event_handler) {
                        warn!("메시지 처리 오류: {:?}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    warn!("WebSocket 연결이 닫혔습니다");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    // Ping에 대한 Pong 응답
                    if let Err(e) = write.send(Message::Pong(data)).await {
                        error!("Pong 전송 실패: {:?}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("WebSocket 메시지 수신 오류: {:?}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// WebSocket API용 서명 생성
    /// signature를 제외한 params를 key 알파벳 순으로 정렬하여 서명
    fn sign_user_data_params(params: &mut BTreeMap<String, String>, secret: &str) -> String {
        // signature 제외한 params를 key 알파벳 순 정렬
        let mut items: Vec<(&String, &String)> = params.iter().collect();
        items.sort_by_key(|(k, _)| *k);

        let payload = items
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        generate_signature(&payload, secret)
    }

    /// User Data Stream 구독 요청 전송
    async fn subscribe_user_data_stream<S>(
        write: &mut S,
        api_key: &str,
        api_secret: &str,
    ) -> Result<String, ExchangeError>
    where
        S: SinkExt<Message> + Unpin,
        <S as futures_util::Sink<Message>>::Error: std::fmt::Debug,
    {
        let timestamp = get_timestamp().to_string();

        let mut params = BTreeMap::new();
        params.insert("apiKey".to_string(), api_key.to_string());
        params.insert("timestamp".to_string(), timestamp);
        // 필요하면 recvWindow 추가 가능
        // params.insert("recvWindow".to_string(), "5000".to_string());

        let signature = Self::sign_user_data_params(&mut params, api_secret);
        params.insert("signature".to_string(), signature);

        let request = WsRequest {
            id: "user-stream-1".to_string(),
            method: "userDataStream.subscribe.signature".to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)
            .map_err(|e| ExchangeError::Other(format!("Failed to serialize request: {}", e)))?;

        info!("Sending subscribe request: {}", request_json);

        write.send(Message::Text(request_json)).await.map_err(|e| {
            ExchangeError::Other(format!("Failed to send subscribe request: {:?}", e))
        })?;

        Ok("user-stream-1".to_string())
    }

    /// 메시지 처리 및 이벤트 파싱
    fn handle_user_data_message<F>(text: &str, event_handler: &mut F) -> Result<(), ExchangeError>
    where
        F: FnMut(UserDataEvent),
    {
        // 먼저 WsResponse로 파싱 시도
        if let Ok(response) = serde_json::from_str::<WsResponse>(text) {
            // 응답 형식인 경우
            if let Some(result) = response.result {
                // result 안에 이벤트가 있을 수 있음
                if let Some(event) = Self::parse_user_data_event(result) {
                    event_handler(event);
                }
            }
            return Ok(());
        }

        // 직접 이벤트 형식인 경우
        if let Ok(event) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(parsed_event) = Self::parse_user_data_event(event) {
                event_handler(parsed_event);
            }
        }

        Ok(())
    }

    /// JSON Value에서 이벤트 파싱
    fn parse_user_data_event(value: serde_json::Value) -> Option<UserDataEvent> {
        // executionReport 이벤트 확인
        if let Some(event_type) = value.get("e").and_then(|v| v.as_str()) {
            match event_type {
                "executionReport" => {
                    if let Ok(report) = serde_json::from_value::<ExecutionReport>(value.clone()) {
                        return Some(UserDataEvent::ExecutionReport(report));
                    } else {
                        warn!("Failed to parse executionReport: {:?}", value);
                    }
                }
                "outboundAccountPosition" => {
                    if let Ok(position) =
                        serde_json::from_value::<OutboundAccountPosition>(value.clone())
                    {
                        return Some(UserDataEvent::OutboundAccountPosition(position));
                    } else {
                        warn!("Failed to parse outboundAccountPosition: {:?}", value);
                    }
                }
                "balanceUpdate" => {
                    if let Ok(update) = serde_json::from_value::<BalanceUpdate>(value.clone()) {
                        return Some(UserDataEvent::BalanceUpdate(update));
                    } else {
                        warn!("Failed to parse balanceUpdate: {:?}", value);
                    }
                }
                _ => {
                    info!("Unknown event type: {}", event_type);
                }
            }
        }

        // 파싱 실패한 경우 Unknown으로 처리
        Some(UserDataEvent::Unknown(value))
    }
}

// ========== User Data Stream 관련 타입 정의 ==========

/// WebSocket API 요청 메시지
#[derive(Debug, Serialize)]
struct WsRequest {
    id: String,
    method: String,
    params: BTreeMap<String, String>,
}

/// WebSocket API 응답 메시지
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WsResponse {
    id: Option<String>,
    status: Option<u16>,
    result: Option<serde_json::Value>,
    error: Option<WsError>,
}

/// WebSocket API 에러
#[derive(Debug, Deserialize)]
struct WsError {
    code: Option<i32>,
    msg: Option<String>,
}

/// 주문 실행 리포트 (executionReport)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReport {
    /// 이벤트 타입
    #[serde(rename = "e")]
    pub event_type: String,
    /// 이벤트 시간
    #[serde(rename = "E")]
    pub event_time: u64,
    /// 심볼
    #[serde(rename = "s")]
    pub symbol: String,
    /// 클라이언트 주문 ID
    #[serde(rename = "c")]
    pub client_order_id: String,
    /// 주문 방향 (BUY/SELL)
    #[serde(rename = "S")]
    pub side: String,
    /// 주문 타입
    #[serde(rename = "o")]
    pub order_type: String,
    /// 시장가 주문 시 사용 (MARKET)
    #[serde(rename = "f")]
    pub time_in_force: String,
    /// 주문 수량
    #[serde(rename = "q")]
    pub order_quantity: String,
    /// 주문 가격
    #[serde(rename = "p")]
    pub order_price: String,
    /// 현재 주문 상태
    #[serde(rename = "X")]
    pub current_order_status: String,
    /// 마지막 실행 수량
    #[serde(rename = "l")]
    pub last_executed_quantity: String,
    /// 누적 실행 수량
    #[serde(rename = "z")]
    pub cumulative_filled_quantity: String,
    /// 마지막 실행 가격
    #[serde(rename = "L")]
    pub last_executed_price: String,
    /// 수수료
    #[serde(rename = "n")]
    pub commission_amount: String,
    /// 수수료 자산
    #[serde(rename = "N")]
    pub commission_asset: Option<String>,
    /// 주문 생성 시간
    #[serde(rename = "O")]
    pub order_create_time: u64,
    /// 거래 ID
    #[serde(rename = "T")]
    pub transaction_time: u64,
    /// 주문 ID
    #[serde(rename = "i")]
    pub order_id: u64,
    /// 누적 인용 수량
    #[serde(rename = "Z")]
    pub cumulative_quote_quantity: Option<String>,
    /// 마지막 인용 수량
    #[serde(rename = "Y")]
    pub last_quote_transacted: Option<String>,
    /// 주문 리스트 ID
    #[serde(rename = "g")]
    pub order_list_id: Option<i64>,
    /// 원래 클라이언트 주문 ID
    #[serde(rename = "C")]
    pub original_client_order_id: Option<String>,
    /// 스톱 가격
    #[serde(rename = "P")]
    pub stop_price: Option<String>,
    /// 거부된 수량
    #[serde(rename = "d")]
    pub rejected_quantity: Option<String>,
    /// 거부된 수량의 원인
    #[serde(rename = "j")]
    pub reject_reason: Option<String>,
}

/// User Data Stream 이벤트 타입
#[derive(Debug, Clone)]
pub enum UserDataEvent {
    ExecutionReport(ExecutionReport),
    OutboundAccountPosition(OutboundAccountPosition),
    BalanceUpdate(BalanceUpdate),
    Unknown(serde_json::Value),
}

/// 계정 정보 업데이트 (outboundAccountPosition)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboundAccountPosition {
    /// 이벤트 타입
    #[serde(rename = "e")]
    pub event_type: String,
    /// 이벤트 시간
    #[serde(rename = "E")]
    pub event_time: u64,
    /// 마지막 업데이트 시간
    #[serde(rename = "u")]
    pub last_update_time: u64,
    /// 잔고 정보
    #[serde(rename = "B")]
    pub balances: Vec<BalanceInfo>,
}

/// 잔고 정보
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceInfo {
    /// 자산
    #[serde(rename = "a")]
    pub asset: String,
    /// 사용 가능한 잔고
    #[serde(rename = "f")]
    pub free: String,
    /// 잠긴 잔고
    #[serde(rename = "l")]
    pub locked: String,
}

/// 잔고 업데이트 (balanceUpdate)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceUpdate {
    /// 이벤트 타입
    #[serde(rename = "e")]
    pub event_type: String,
    /// 이벤트 시간
    #[serde(rename = "E")]
    pub event_time: u64,
    /// 자산
    #[serde(rename = "a")]
    pub asset: String,
    /// 잔고 변화량
    #[serde(rename = "d")]
    pub balance_delta: String,
    /// 지갑 타입
    #[serde(rename = "w")]
    pub wallet_type: Option<String>,
}

