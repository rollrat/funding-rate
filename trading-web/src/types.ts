// entities.rs의 trade_record 모델 기반 타입
export interface TradeRecord {
  id: number;
  executed_at: string; // ISO 8601 형식
  exchange: string;
  symbol: string;
  market_type: string; // "SPOT" | "FUTURES"
  side: string; // "BUY" | "SELL"
  trade_type: string; // "MARKET" | "LIMIT" | "OTHER"
  executed_price: number | null;
  quantity: number;
  request_query_string: string | null;
  api_response: string | null;
  metadata: string | null;
  is_liquidation: boolean;
}

// entities.rs의 position_record 모델 기반 타입
export interface PositionRecord {
  id: number;
  executed_at: string; // ISO 8601 형식
  bot_name: string;
  carry: string; // "CARRY" | "REVERSE"
  action: string; // "OPEN" | "CLOSE"
  symbol: string;
  spot_price: number;
  futures_mark: number;
  buy_exchange: string;
  sell_exchange: string;
}

// 콘솔 로그 타입
export interface ConsoleLog {
  timestamp: string;
  level: "info" | "warn" | "error" | "success";
  message: string;
}

