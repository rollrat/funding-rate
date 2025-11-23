import { useState, useMemo, useRef, memo, useCallback } from "react";
import { Table, Text, Badge, Group, Center, Button } from "@mantine/core";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  IconChevronUp,
  IconChevronDown,
  IconArrowsUpDown,
  IconChartLine,
  IconCoin,
} from "@tabler/icons-react";
import type { UnifiedSnapshot, SortField, SortDirection } from "../types";

interface SnapshotTableProps {
  snapshots: UnifiedSnapshot[];
}

// Row 컴포넌트를 메모이제이션하여 불필요한 리렌더링 방지
const TableRow = memo(
  ({
    snapshot,
    formatSymbol,
    formatOI,
    formatVol,
    formatFundingRate,
    formatPerpSpotGap,
    getTimeUntilFunding,
    getExchangeBadgeColor,
    getFundingRateColor,
    getExchangeUrl,
    getSpotUrl,
    hasSpotData,
  }: {
    snapshot: UnifiedSnapshot;
    formatSymbol: (symbol: string) => string;
    formatOI: (oi: number) => string;
    formatVol: (vol: number) => string;
    formatFundingRate: (rate: number) => string;
    formatPerpSpotGap: (perpPrice: number, spotPrice: number | null) => string;
    getTimeUntilFunding: (nextFundingTime: string | null) => string;
    getExchangeBadgeColor: (exchange: string) => string;
    getFundingRateColor: (rate: number) => string;
    getExchangeUrl: (exchange: string, symbol: string) => string;
    getSpotUrl: (exchange: string, symbol: string) => string;
    hasSpotData: boolean;
  }) => {
    const perp = snapshot.perp!; // perp가 없는 경우는 필터링되어 있음

    return (
      <Table.Tr>
        <Table.Td
          style={{
            width: "100px",
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
        >
          <Badge
            color={getExchangeBadgeColor(snapshot.exchange)}
            variant="light"
          >
            {snapshot.exchange}
          </Badge>
        </Table.Td>
        <Table.Td
          style={{
            width: "120px",
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
        >
          <Text ff="monospace" fw={500} truncate>
            {formatSymbol(snapshot.symbol)}
          </Text>
        </Table.Td>
        <Table.Td style={{ width: "140px" }}>
          <Text fw={500}>
            $
            {perp.mark_price.toLocaleString("en-US", {
              minimumFractionDigits: 2,
              maximumFractionDigits: 2,
            })}
          </Text>
        </Table.Td>
        {hasSpotData && (
          <Table.Td style={{ width: "140px" }}>
            {snapshot.spot ? (
              <Text fw={500} c="dimmed">
                $
                {snapshot.spot.price.toLocaleString("en-US", {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 2,
                })}
              </Text>
            ) : (
              <Text c="dimmed">-</Text>
            )}
          </Table.Td>
        )}
        {hasSpotData && (
          <Table.Td style={{ width: "130px" }}>
            {snapshot.spot ? (
              <Badge
                color={getFundingRateColor(
                  (perp.mark_price - snapshot.spot.price) / snapshot.spot.price
                )}
                variant="light"
              >
                {formatPerpSpotGap(perp.mark_price, snapshot.spot.price)}
              </Badge>
            ) : (
              <Text c="dimmed">-</Text>
            )}
          </Table.Td>
        )}
        <Table.Td style={{ width: "130px" }}>
          <Text fw={500}>{formatOI(perp.oi_usd)}M</Text>
        </Table.Td>
        <Table.Td style={{ width: "120px" }}>
          <Text>${formatVol(perp.vol_24h_usd)}</Text>
        </Table.Td>
        <Table.Td style={{ width: "130px" }}>
          <Badge color={getFundingRateColor(perp.funding_rate)} variant="light">
            {formatFundingRate(perp.funding_rate)}
          </Badge>
        </Table.Td>
        <Table.Td style={{ width: "150px" }}>
          <Text size="sm">{getTimeUntilFunding(perp.next_funding_time)}</Text>
        </Table.Td>
        <Table.Td style={{ width: "120px" }}>
          <div
            style={{
              display: "flex",
              gap: "4px",
              flexDirection: "row",
              alignItems: "center",
            }}
          >
            <Button
              component="a"
              href={getExchangeUrl(snapshot.exchange, snapshot.symbol)}
              target="_blank"
              rel="noopener noreferrer"
              size="xs"
              variant="light"
              p={4}
              style={{ minWidth: "auto", width: "auto", height: "auto" }}
            >
              <IconChartLine size={16} />
            </Button>
            {snapshot.spot && (
              <Button
                component="a"
                href={getSpotUrl(snapshot.exchange, snapshot.symbol)}
                target="_blank"
                rel="noopener noreferrer"
                size="xs"
                variant="light"
                color="green"
                p={4}
                style={{ minWidth: "auto", width: "auto", height: "auto" }}
              >
                <IconCoin size={16} />
              </Button>
            )}
          </div>
        </Table.Td>
      </Table.Tr>
    );
  }
);

TableRow.displayName = "TableRow";

const SnapshotTable = ({ snapshots }: SnapshotTableProps) => {
  const [sortField, setSortField] = useState<SortField>("funding_rate");
  const [sortDirection, setSortDirection] = useState<SortDirection>("desc");
  const parentRef = useRef<HTMLDivElement>(null);

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection(sortDirection === "asc" ? "desc" : "asc");
    } else {
      setSortField(field);
      setSortDirection("desc");
    }
  };

  const sortedSnapshots = useMemo(() => {
    const sorted = [...snapshots];
    sorted.sort((a, b) => {
      let aValue: number | string;
      let bValue: number | string;

      switch (sortField) {
        case "exchange":
          aValue = a.exchange;
          bValue = b.exchange;
          break;
        case "symbol":
          aValue = a.symbol;
          bValue = b.symbol;
          break;
        case "mark_price":
          aValue = a.perp?.mark_price ?? 0;
          bValue = b.perp?.mark_price ?? 0;
          break;
        case "spot_price":
          aValue = a.spot?.price ?? 0;
          bValue = b.spot?.price ?? 0;
          break;
        case "perp_spot_gap":
          const aGap =
            a.perp && a.spot
              ? (a.perp.mark_price - a.spot.price) / a.spot.price
              : 0;
          const bGap =
            b.perp && b.spot
              ? (b.perp.mark_price - b.spot.price) / b.spot.price
              : 0;
          aValue = aGap;
          bValue = bGap;
          break;
        case "oi_usd":
          aValue = a.perp?.oi_usd ?? 0;
          bValue = b.perp?.oi_usd ?? 0;
          break;
        case "vol_24h_usd":
          aValue = a.perp?.vol_24h_usd ?? 0;
          bValue = b.perp?.vol_24h_usd ?? 0;
          break;
        case "funding_rate":
          aValue = a.perp?.funding_rate ?? 0;
          bValue = b.perp?.funding_rate ?? 0;
          break;
        case "next_funding_time":
          aValue = a.perp?.next_funding_time || "";
          bValue = b.perp?.next_funding_time || "";
          break;
        default:
          return 0;
      }

      if (typeof aValue === "string" && typeof bValue === "string") {
        return sortDirection === "asc"
          ? aValue.localeCompare(bValue)
          : bValue.localeCompare(aValue);
      } else {
        return sortDirection === "asc"
          ? (aValue as number) - (bValue as number)
          : (bValue as number) - (aValue as number);
      }
    });
    return sorted;
  }, [snapshots, sortField, sortDirection]);

  // 포맷팅 함수들을 useCallback으로 메모이제이션
  const formatOI = useCallback((oi: number): string => {
    return (oi / 1_000_000).toFixed(2);
  }, []);

  const formatVol = useCallback((vol: number): string => {
    if (vol >= 1_000_000_000) {
      return (vol / 1_000_000_000).toFixed(2) + "B";
    } else if (vol >= 1_000_000) {
      return (vol / 1_000_000).toFixed(2) + "M";
    } else if (vol >= 1_000) {
      return (vol / 1_000).toFixed(2) + "K";
    }
    return vol.toFixed(2);
  }, []);

  const formatFundingRate = useCallback((rate: number): string => {
    return (rate * 100).toFixed(4) + "%";
  }, []);

  const formatPerpSpotGap = useCallback(
    (perpPrice: number, spotPrice: number | null): string => {
      if (!spotPrice || spotPrice === 0) return "-";
      const gap = ((perpPrice - spotPrice) / spotPrice) * 100;
      return (gap > 0 ? "+" : "") + gap.toFixed(4) + "%";
    },
    []
  );

  const formatSymbol = useCallback((symbol: string): string => {
    if (symbol.endsWith("USDT")) {
      return symbol.slice(0, -4) + "/USDT";
    }
    return symbol;
  }, []);

  const getTimeUntilFunding = useCallback(
    (nextFundingTime: string | null): string => {
      if (!nextFundingTime) return "-";

      const now = new Date();
      const next = new Date(nextFundingTime);
      const diff = next.getTime() - now.getTime();

      if (diff < 0) return "지남";

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
    },
    []
  );

  const getSortIcon = (field: SortField) => {
    if (sortField !== field) {
      return <IconArrowsUpDown size={14} />;
    }
    return sortDirection === "asc" ? (
      <IconChevronUp size={14} />
    ) : (
      <IconChevronDown size={14} />
    );
  };

  const getExchangeBadgeColor = useCallback((exchange: string) => {
    switch (exchange) {
      case "Binance":
        return "yellow";
      case "Bybit":
        return "orange";
      case "Okx":
        return "gray";
      case "Bitget":
        return "cyan";
      default:
        return "gray";
    }
  }, []);

  const getFundingRateColor = useCallback((rate: number) => {
    if (rate > 0.01) return "red";
    if (rate > 0.005) return "orange";
    if (rate < -0.01) return "green";
    if (rate < -0.005) return "lime";
    return "gray";
  }, []);

  const getExchangeUrl = useCallback(
    (exchange: string, symbol: string): string => {
      switch (exchange) {
        case "Binance":
          return `https://www.binance.com/en/futures/${symbol}`;
        case "Bybit":
          return `https://www.bybit.com/trade/usdt/${symbol}`;
        case "Okx":
          // 심볼에서 USDT를 제거하고 나머지를 소문자로 변환
          const baseSymbol = symbol.replace(/USDT$/i, "").toLowerCase();
          return `https://www.okx.com/trade-swap/${baseSymbol}-usdt-swap`;
        case "Bitget":
          return `https://www.bitget.com/futures/usdt/${symbol}`;
        default:
          return "#";
      }
    },
    []
  );

  const getSpotUrl = useCallback((exchange: string, symbol: string): string => {
    switch (exchange) {
      case "Binance":
        return `https://www.binance.com/en/trade/${symbol}`;
      case "Bybit":
        return `https://www.bybit.com/trade/usdt/${symbol}`;
      case "Okx":
        const baseSymbol = symbol.replace(/USDT$/i, "").toLowerCase();
        return `https://www.okx.com/trade-spot/${baseSymbol}-usdt`;
      case "Bitget":
        return `https://www.bitget.com/spot/${symbol}`;
      default:
        return "#";
    }
  }, []);

  // spot 데이터가 있는 항목이 하나라도 있는지 확인
  const hasSpotData = useMemo(() => {
    return snapshots.some((snapshot) => snapshot.spot !== null);
  }, [snapshots]);

  // 가상화 설정
  const rowVirtualizer = useVirtualizer({
    count: sortedSnapshots.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 40, // 각 row의 예상 높이
    overscan: 10, // 화면 밖에 미리 렌더링할 row 수
    measureElement: (element) => element?.getBoundingClientRect().height ?? 40,
  });

  const virtualItems = rowVirtualizer.getVirtualItems();
  const totalSize = rowVirtualizer.getTotalSize();
  const paddingTop = virtualItems.length > 0 ? virtualItems[0]?.start ?? 0 : 0;
  const paddingBottom =
    virtualItems.length > 0
      ? Math.max(
          0,
          totalSize - (virtualItems[virtualItems.length - 1]?.end ?? 0)
        )
      : 0;

  if (sortedSnapshots.length === 0) {
    return (
      <Center py="xl">
        <Text c="dimmed">데이터가 없습니다</Text>
      </Center>
    );
  }

  return (
    <div style={{ height: "100%", overflow: "auto" }} ref={parentRef}>
      <Table.ScrollContainer minWidth={0} className="fixed-width-table">
        <Table
          highlightOnHover
          striped
          style={{ tableLayout: "auto", width: "100%" }}
        >
          <style>{`
            table tbody tr td {
              padding: 4px 8px !important;
            }
            table thead tr th {
              padding: 8px 12px !important;
            }
          `}</style>
          <Table.Thead
            style={{
              position: "sticky",
              top: 0,
              zIndex: 1,
              backgroundColor: "var(--mantine-color-dark-7)",
            }}
          >
            <Table.Tr>
              <Table.Th
                style={{ cursor: "pointer", width: "100px" }}
                onClick={() => handleSort("exchange")}
              >
                <Group gap={4}>
                  거래소
                  {getSortIcon("exchange")}
                </Group>
              </Table.Th>
              <Table.Th
                style={{ cursor: "pointer", width: "120px" }}
                onClick={() => handleSort("symbol")}
              >
                <Group gap={4}>
                  심볼
                  {getSortIcon("symbol")}
                </Group>
              </Table.Th>
              <Table.Th
                style={{ cursor: "pointer", width: "140px" }}
                onClick={() => handleSort("mark_price")}
              >
                <Group gap={4}>
                  마크프라이스
                  {getSortIcon("mark_price")}
                </Group>
              </Table.Th>
              {hasSpotData && (
                <Table.Th
                  style={{ cursor: "pointer", width: "140px" }}
                  onClick={() => handleSort("spot_price")}
                >
                  <Group gap={4}>
                    현물 가격
                    {getSortIcon("spot_price")}
                  </Group>
                </Table.Th>
              )}
              {hasSpotData && (
                <Table.Th
                  style={{ cursor: "pointer", width: "130px" }}
                  onClick={() => handleSort("perp_spot_gap")}
                >
                  <Group gap={4}>선현물 갭{getSortIcon("perp_spot_gap")}</Group>
                </Table.Th>
              )}
              <Table.Th
                style={{ cursor: "pointer", width: "130px" }}
                onClick={() => handleSort("oi_usd")}
              >
                <Group gap={4}>
                  OI (백만달러)
                  {getSortIcon("oi_usd")}
                </Group>
              </Table.Th>
              <Table.Th
                style={{ cursor: "pointer", width: "120px" }}
                onClick={() => handleSort("vol_24h_usd")}
              >
                <Group gap={4}>
                  24h Vol
                  {getSortIcon("vol_24h_usd")}
                </Group>
              </Table.Th>
              <Table.Th
                style={{ cursor: "pointer", width: "130px" }}
                onClick={() => handleSort("funding_rate")}
              >
                <Group gap={4}>
                  펀딩 비율 (%)
                  {getSortIcon("funding_rate")}
                </Group>
              </Table.Th>
              <Table.Th
                style={{ cursor: "pointer", width: "150px" }}
                onClick={() => handleSort("next_funding_time")}
              >
                <Group gap={4}>
                  다음 펀딩까지
                  {getSortIcon("next_funding_time")}
                </Group>
              </Table.Th>
              <Table.Th style={{ width: "120px" }}>액션</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody
            style={{ height: `${totalSize}px`, position: "relative" }}
          >
            {paddingTop > 0 && (
              <Table.Tr>
                <Table.Td
                  colSpan={hasSpotData ? 10 : 8}
                  style={{ height: `${paddingTop}px`, padding: 0 }}
                />
              </Table.Tr>
            )}
            {virtualItems.map((virtualRow) => {
              const snapshot = sortedSnapshots[virtualRow.index];
              return (
                <TableRow
                  key={`${snapshot.exchange}-${snapshot.symbol}-${virtualRow.index}`}
                  snapshot={snapshot}
                  formatSymbol={formatSymbol}
                  formatOI={formatOI}
                  formatVol={formatVol}
                  formatFundingRate={formatFundingRate}
                  formatPerpSpotGap={formatPerpSpotGap}
                  getTimeUntilFunding={getTimeUntilFunding}
                  getExchangeBadgeColor={getExchangeBadgeColor}
                  getFundingRateColor={getFundingRateColor}
                  getExchangeUrl={getExchangeUrl}
                  getSpotUrl={getSpotUrl}
                  hasSpotData={hasSpotData}
                />
              );
            })}
            {paddingBottom > 0 && (
              <Table.Tr>
                <Table.Td
                  colSpan={hasSpotData ? 10 : 8}
                  style={{ height: `${paddingBottom}px`, padding: 0 }}
                />
              </Table.Tr>
            )}
          </Table.Tbody>
        </Table>
      </Table.ScrollContainer>
    </div>
  );
};

export default SnapshotTable;
