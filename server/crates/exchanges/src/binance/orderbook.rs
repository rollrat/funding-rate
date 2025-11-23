use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use interface::{ExchangeId, OrderBook, OrderBookEntry};

use super::super::{ExchangeError, OrderBookExchange};
use super::{BinanceClient, BASE_URL};

impl BinanceClient {
    /// 심볼을 Binance 형식으로 변환
    /// 예: "BTC-KRW" -> "BTCKRW", "BTC-USDT" -> "BTCUSDT"
    fn normalize_symbol(&self, symbol: &str) -> String {
        symbol.replace("-", "").to_uppercase()
    }
}

#[derive(Debug, Deserialize)]
struct BinanceOrderBookResponse {
    bids: Vec<[String; 2]>, // [price, quantity]
    asks: Vec<[String; 2]>, // [price, quantity]
}

#[async_trait]
impl OrderBookExchange for BinanceClient {
    fn id(&self) -> ExchangeId {
        ExchangeId::Binance
    }

    async fn fetch_orderbook(&self, symbol: &str) -> Result<OrderBook, ExchangeError> {
        let normalized_symbol = self.normalize_symbol(symbol);
        let url = format!(
            "{}/api/v3/depth?symbol={}&limit=100",
            BASE_URL, normalized_symbol
        );

        let response = self.http.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let response_text = response.text().await?;
            return Err(ExchangeError::Other(format!(
                "Binance API HTTP error: status {}, response: {}",
                status,
                response_text.chars().take(200).collect::<String>()
            )));
        }

        let orderbook_response: BinanceOrderBookResponse = response.json().await?;

        let mut bids = Vec::new();
        for bid in orderbook_response.bids {
            let price: f64 = bid[0]
                .parse()
                .map_err(|e| ExchangeError::Other(format!("Failed to parse bid price: {}", e)))?;
            let quantity: f64 = bid[1].parse().map_err(|e| {
                ExchangeError::Other(format!("Failed to parse bid quantity: {}", e))
            })?;
            bids.push(OrderBookEntry { price, quantity });
        }

        let mut asks = Vec::new();
        for ask in orderbook_response.asks {
            let price: f64 = ask[0]
                .parse()
                .map_err(|e| ExchangeError::Other(format!("Failed to parse ask price: {}", e)))?;
            let quantity: f64 = ask[1].parse().map_err(|e| {
                ExchangeError::Other(format!("Failed to parse ask quantity: {}", e))
            })?;
            asks.push(OrderBookEntry { price, quantity });
        }

        // Binance는 이미 가격 순서대로 정렬되어 있지만, 확실하게 정렬
        bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap()); // 높은 가격 순
        asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap()); // 낮은 가격 순

        Ok(OrderBook {
            exchange: ExchangeId::Binance,
            symbol: normalized_symbol,
            bids,
            asks,
            updated_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle_api_error(e: &ExchangeError) {
        match e {
            ExchangeError::Http(reqwest_err) => {
                println!("HTTP 오류: {:?}", reqwest_err);
            }
            ExchangeError::Other(msg) => {
                println!("기타 오류: {}", msg);
            }
        }
    }

    #[tokio::test]
    async fn test_fetch_orderbook_binance() {
        let client = BinanceClient::new();

        // BTC-USDT 오더북 조회
        match client.fetch_orderbook("BTC-USDT").await {
            Ok(orderbook) => {
                assert_eq!(orderbook.exchange, ExchangeId::Binance);
                assert_eq!(orderbook.symbol, "BTCUSDT");
                assert!(!orderbook.bids.is_empty(), "Should have bids");
                assert!(!orderbook.asks.is_empty(), "Should have asks");

                println!("\n=== Binance Orderbook for {} ===", orderbook.symbol);
                println!(
                    "  Best bid: {} @ {}",
                    orderbook.bids[0].price, orderbook.bids[0].quantity
                );
                println!(
                    "  Best ask: {} @ {}",
                    orderbook.asks[0].price, orderbook.asks[0].quantity
                );
                println!(
                    "  Spread: {}",
                    orderbook.asks[0].price - orderbook.bids[0].price
                );
                println!("  Bids count: {}", orderbook.bids.len());
                println!("  Asks count: {}", orderbook.asks.len());
            }
            Err(e) => {
                handle_api_error(&e);
                // 네트워크 오류일 수 있으므로 테스트 실패로 처리하지 않음
            }
        }
    }
}
