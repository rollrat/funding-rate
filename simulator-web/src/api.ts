import type { OrderBookResponse, Trade, OrderRequest, OrderResponse } from './types';

const API_BASE = '/api';

export async function fetchOrderBook(): Promise<OrderBookResponse> {
  const response = await fetch(`${API_BASE}/orderbook`);
  if (!response.ok) {
    throw new Error(`Failed to fetch orderbook: ${response.statusText}`);
  }
  return response.json();
}

export async function fetchTrades(): Promise<Trade[]> {
  const response = await fetch(`${API_BASE}/trades`);
  if (!response.ok) {
    throw new Error(`Failed to fetch trades: ${response.statusText}`);
  }
  return response.json();
}

export async function submitOrder(order: OrderRequest): Promise<OrderResponse> {
  const response = await fetch(`${API_BASE}/order`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(order),
  });
  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Failed to submit order: ${error}`);
  }
  return response.json();
}

