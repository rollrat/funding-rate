export type ExchangeId = 'Binance' | 'Bybit' | 'Okx';

export interface PerpSnapshot {
    exchange: ExchangeId;
    symbol: string;
    mark_price: number;
    oi_usd: number;
    vol_24h_usd: number;
    funding_rate: number;
    next_funding_time: string;
    updated_at: string;
}

export type SortField = 'exchange' | 'symbol' | 'mark_price' | 'oi_usd' | 'vol_24h_usd' | 'funding_rate' | 'next_funding_time';
export type SortDirection = 'asc' | 'desc';

