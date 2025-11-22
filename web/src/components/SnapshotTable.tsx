import { useState, useMemo } from 'react';
import type { PerpSnapshot, SortField, SortDirection } from '../types';

interface SnapshotTableProps {
    snapshots: PerpSnapshot[];
}

const SnapshotTable = ({ snapshots }: SnapshotTableProps) => {
    const [sortField, setSortField] = useState<SortField>('oi_usd');
    const [sortDirection, setSortDirection] = useState<SortDirection>('desc');

    const handleSort = (field: SortField) => {
        if (sortField === field) {
            setSortDirection(sortDirection === 'asc' ? 'desc' : 'asc');
        } else {
            setSortField(field);
            setSortDirection('desc');
        }
    };

    const sortedSnapshots = useMemo(() => {
        const sorted = [...snapshots];
        sorted.sort((a, b) => {
            let aValue: number | string;
            let bValue: number | string;

            switch (sortField) {
                case 'exchange':
                    aValue = a.exchange;
                    bValue = b.exchange;
                    break;
                case 'symbol':
                    aValue = a.symbol;
                    bValue = b.symbol;
                    break;
                case 'mark_price':
                    aValue = a.mark_price;
                    bValue = b.mark_price;
                    break;
                case 'oi_usd':
                    aValue = a.oi_usd;
                    bValue = b.oi_usd;
                    break;
                case 'vol_24h_usd':
                    aValue = a.vol_24h_usd;
                    bValue = b.vol_24h_usd;
                    break;
                case 'funding_rate':
                    aValue = a.funding_rate;
                    bValue = b.funding_rate;
                    break;
                case 'next_funding_time':
                    aValue = a.next_funding_time || '';
                    bValue = b.next_funding_time || '';
                    break;
                default:
                    return 0;
            }

            if (typeof aValue === 'string' && typeof bValue === 'string') {
                return sortDirection === 'asc'
                    ? aValue.localeCompare(bValue)
                    : bValue.localeCompare(aValue);
            } else {
                return sortDirection === 'asc'
                    ? (aValue as number) - (bValue as number)
                    : (bValue as number) - (aValue as number);
            }
        });
        return sorted;
    }, [snapshots, sortField, sortDirection]);

    const formatOI = (oi: number): string => {
        return (oi / 1_000_000).toFixed(2);
    };

    const formatVol = (vol: number): string => {
        if (vol >= 1_000_000_000) {
            return (vol / 1_000_000_000).toFixed(2) + 'B';
        } else if (vol >= 1_000_000) {
            return (vol / 1_000_000).toFixed(2) + 'M';
        } else if (vol >= 1_000) {
            return (vol / 1_000).toFixed(2) + 'K';
        }
        return vol.toFixed(2);
    };

    const formatFundingRate = (rate: number): string => {
        return (rate * 100).toFixed(4) + '%';
    };

    const getTimeUntilFunding = (nextFundingTime: string): string => {
        if (!nextFundingTime) return '-';

        const now = new Date();
        const next = new Date(nextFundingTime);
        const diff = next.getTime() - now.getTime();

        if (diff < 0) return '지남';

        const hours = Math.floor(diff / (1000 * 60 * 60));
        const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
        const seconds = Math.floor((diff % (1000 * 60)) / 1000);

        if (hours > 0) {
            return `${hours}시간 ${minutes}분`;
        } else if (minutes > 0) {
            return `${minutes}분 ${seconds}초`;
        } else {
            return `${seconds}초`;
        }
    };

    const SortIcon = ({ field }: { field: SortField }) => {
        if (sortField !== field) {
            return <span className="text-slate-500">↕</span>;
        }
        return sortDirection === 'asc' ? (
            <span className="text-blue-400">↑</span>
        ) : (
            <span className="text-blue-400">↓</span>
        );
    };

    return (
        <div className="overflow-x-auto">
            <table className="w-full border-collapse bg-slate-800 rounded-lg overflow-hidden">
                <thead>
                    <tr className="bg-slate-700">
                        <th
                            className="px-4 py-3 text-left text-sm font-semibold text-slate-200 cursor-pointer hover:bg-slate-600 transition-colors"
                            onClick={() => handleSort('exchange')}
                        >
                            <div className="flex items-center gap-2">
                                거래소
                                <SortIcon field="exchange" />
                            </div>
                        </th>
                        <th
                            className="px-4 py-3 text-left text-sm font-semibold text-slate-200 cursor-pointer hover:bg-slate-600 transition-colors"
                            onClick={() => handleSort('symbol')}
                        >
                            <div className="flex items-center gap-2">
                                심볼
                                <SortIcon field="symbol" />
                            </div>
                        </th>
                        <th
                            className="px-4 py-3 text-left text-sm font-semibold text-slate-200 cursor-pointer hover:bg-slate-600 transition-colors"
                            onClick={() => handleSort('mark_price')}
                        >
                            <div className="flex items-center gap-2">
                                마크프라이스
                                <SortIcon field="mark_price" />
                            </div>
                        </th>
                        <th
                            className="px-4 py-3 text-left text-sm font-semibold text-slate-200 cursor-pointer hover:bg-slate-600 transition-colors"
                            onClick={() => handleSort('oi_usd')}
                        >
                            <div className="flex items-center gap-2">
                                OI (백만달러)
                                <SortIcon field="oi_usd" />
                            </div>
                        </th>
                        <th
                            className="px-4 py-3 text-left text-sm font-semibold text-slate-200 cursor-pointer hover:bg-slate-600 transition-colors"
                            onClick={() => handleSort('vol_24h_usd')}
                        >
                            <div className="flex items-center gap-2">
                                24h Vol
                                <SortIcon field="vol_24h_usd" />
                            </div>
                        </th>
                        <th
                            className="px-4 py-3 text-left text-sm font-semibold text-slate-200 cursor-pointer hover:bg-slate-600 transition-colors"
                            onClick={() => handleSort('funding_rate')}
                        >
                            <div className="flex items-center gap-2">
                                펀딩 비율 (%)
                                <SortIcon field="funding_rate" />
                            </div>
                        </th>
                        <th
                            className="px-4 py-3 text-left text-sm font-semibold text-slate-200 cursor-pointer hover:bg-slate-600 transition-colors"
                            onClick={() => handleSort('next_funding_time')}
                        >
                            <div className="flex items-center gap-2">
                                다음 펀딩까지
                                <SortIcon field="next_funding_time" />
                            </div>
                        </th>
                    </tr>
                </thead>
                <tbody>
                    {sortedSnapshots.map((snapshot, index) => (
                        <tr
                            key={`${snapshot.exchange}-${snapshot.symbol}-${index}`}
                            className="border-t border-slate-700 hover:bg-slate-750 transition-colors"
                        >
                            <td className="px-4 py-3 text-sm text-slate-300">{snapshot.exchange}</td>
                            <td className="px-4 py-3 text-sm text-slate-300 font-mono">{snapshot.symbol}</td>
                            <td className="px-4 py-3 text-sm text-slate-300">
                                ${snapshot.mark_price.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                            </td>
                            <td className="px-4 py-3 text-sm text-slate-300">
                                {formatOI(snapshot.oi_usd)}M
                            </td>
                            <td className="px-4 py-3 text-sm text-slate-300">
                                ${formatVol(snapshot.vol_24h_usd)}
                            </td>
                            <td className="px-4 py-3 text-sm text-slate-300">
                                {formatFundingRate(snapshot.funding_rate)}
                            </td>
                            <td className="px-4 py-3 text-sm text-slate-300">
                                {getTimeUntilFunding(snapshot.next_funding_time)}
                            </td>
                        </tr>
                    ))}
                </tbody>
            </table>
            {sortedSnapshots.length === 0 && (
                <div className="text-center py-8 text-slate-400">
                    데이터가 없습니다
                </div>
            )}
        </div>
    );
};

export default SnapshotTable;

