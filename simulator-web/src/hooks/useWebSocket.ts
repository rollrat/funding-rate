import { useEffect, useRef, useState } from 'react';
import type { OrderBookResponse, Trade } from '../types';

export interface WebSocketMessage {
  OrderBook?: OrderBookResponse;
  Trades?: Trade[];
}

export function useWebSocket(url: string) {
  const [orderBook, setOrderBook] = useState<OrderBookResponse | null>(null);
  const [trades, setTrades] = useState<Trade[]>([]);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);
  const initialTradesReceivedRef = useRef(false);

  useEffect(() => {
    const connect = () => {
      try {
        const ws = new WebSocket(url);
        wsRef.current = ws;

        ws.onopen = () => {
          console.log('WebSocket connected');
          setConnected(true);
          setError(null);
          // 재연결 시에도 기존 trades 유지 (초기화하지 않음)
          // initialTradesReceivedRef는 첫 연결 시에만 false로 시작
          if (reconnectTimeoutRef.current) {
            clearTimeout(reconnectTimeoutRef.current);
            reconnectTimeoutRef.current = null;
          }
        };

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            
            // 서버에서 보내는 형식: { "OrderBook": {...} } 또는 { "Trades": [...] }
            if (data.OrderBook) {
              setOrderBook(data.OrderBook);
            }
            
            if (data.Trades) {
              if (!initialTradesReceivedRef.current) {
                // 초기 연결 시: 전체 trades를 설정 (최근 100개만)
                const limitedTrades = data.Trades.slice(-100);
                setTrades(limitedTrades);
                initialTradesReceivedRef.current = true;
              } else {
                // 이후: 새로운 trades만 추가하고 최근 100개만 유지
                // 기존 trades와 새 trades를 합치되, 중복 제거 (timestamp로 비교)
                setTrades((prevTrades) => {
                  const existingTimestamps = new Set(
                    prevTrades.map(t => t.timestamp)
                  );
                  const newTrades = data.Trades.filter(
                    t => !existingTimestamps.has(t.timestamp)
                  );
                  const updated = [...prevTrades, ...newTrades];
                  // 최근 100개만 유지
                  return updated.slice(-100);
                });
              }
            }
          } catch (err) {
            console.error('Failed to parse WebSocket message:', err);
          }
        };

        ws.onerror = (err) => {
          console.error('WebSocket error:', err);
          setError('WebSocket 연결 오류');
          setConnected(false);
        };

        ws.onclose = () => {
          console.log('WebSocket disconnected');
          setConnected(false);
          wsRef.current = null;

          // 재연결 시도 (3초 후)
          if (!reconnectTimeoutRef.current) {
            reconnectTimeoutRef.current = window.setTimeout(() => {
              reconnectTimeoutRef.current = null;
              connect();
            }, 3000);
          }
        };
      } catch (err) {
        console.error('Failed to create WebSocket:', err);
        setError('WebSocket 생성 실패');
        setConnected(false);
      }
    };

    connect();

    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [url]);

  return { orderBook, trades, connected, error };
}

