export type ExchangeId = 'Binance' | 'Bybit' | 'Okx' | 'Bitget';

export interface PerpData {
  mark_price: number;
  oi_usd: number;
  vol_24h_usd: number;
  funding_rate: number;
  next_funding_time: string | null;
}

export interface SpotData {
  price: number;
  vol_24h_usd: number;
}

export interface UnifiedSnapshot {
  exchange: ExchangeId;
  symbol: string;
  perp: PerpData | null;
  spot: SpotData | null;
  updated_at: string;
}

export type SortField = 'exchange' | 'symbol' | 'mark_price' | 'spot_price' | 'perp_spot_gap' | 'oi_usd' | 'vol_24h_usd' | 'funding_rate' | 'next_funding_time';
export type SortDirection = 'asc' | 'desc';

