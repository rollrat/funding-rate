export interface Order {
  id: string;
  side: "Buy" | "Sell";
  order_type: "Limit" | "Market";
  price: number | null;
  quantity: number;
  timestamp: string;
}

export interface Trade {
  price: number;
  quantity: number;
  side: "Buy" | "Sell";
  timestamp: string;
}

export interface OrderBookResponse {
  bids: Order[];
  asks: Order[];
}

export interface OrderRequest {
  side: "Buy" | "Sell";
  order_type: "Limit" | "Market";
  price?: number;
  quantity: number;
}

export interface OrderResponse {
  id: string;
  status: "Open" | "Filled" | "PartiallyFilled" | "NotFilled";
  trades: Trade[];
}

export interface CandleData {
  time: number; // Unix timestamp
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

