use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::ExchangeError;

pub mod asset;
pub mod fee;
pub mod orderbook;

pub const BASE_URL: &str = "https://api.binance.com";
pub const SAPI_BASE_URL: &str = "https://api.binance.com";

/// Binance 통합 클라이언트 (Orderbook, Asset, Fee 모두 지원)
#[derive(Clone)]
pub struct BinanceClient {
    pub(crate) http: reqwest::Client,
    pub(crate) api_key: Option<String>,
    pub(crate) api_secret: Option<String>,
}

impl BinanceClient {
    /// 공개 API만 사용하는 경우 (Orderbook 등)
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: None,
            api_secret: None,
        }
    }

    /// 인증이 필요한 API를 사용하는 경우 (Asset, Fee 등)
    pub fn with_credentials() -> Result<Self, ExchangeError> {
        let (api_key, api_secret) = get_api_credentials()?;
        Ok(Self {
            http: reqwest::Client::new(),
            api_key: Some(api_key),
            api_secret: Some(api_secret),
        })
    }
}

type HmacSha256 = Hmac<Sha256>;

/// Binance API 서명 생성
/// query_string: 쿼리 파라미터 문자열 (예: "symbol=BTCUSDT&timestamp=1234567890")
/// api_secret: API Secret Key
pub fn generate_signature(query_string: &str, api_secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(api_secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(query_string.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// 타임스탬프 생성 (밀리초)
pub fn get_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// 환경변수에서 API 키와 시크릿 가져오기
pub fn get_api_credentials() -> Result<(String, String), ExchangeError> {
    let api_key = env::var("BINANCE_API_KEY")
        .map_err(|e| ExchangeError::Other(format!("BINANCE_API_KEY not found: {}", e)))?;
    let api_secret = env::var("BINANCE_API_SECRET")
        .map_err(|e| ExchangeError::Other(format!("BINANCE_API_SECRET not found: {}", e)))?;
    Ok((api_key, api_secret))
}

/// 환경변수가 설정되어 있는지 확인
pub fn has_api_credentials() -> bool {
    env::var("BINANCE_API_KEY").is_ok() && env::var("BINANCE_API_SECRET").is_ok()
}

// BinanceClient는 mod.rs에 정의되어 있음
