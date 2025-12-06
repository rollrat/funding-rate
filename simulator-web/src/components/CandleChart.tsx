import { useEffect, useRef } from "react";
import {
  createChart,
  IChartApi,
  ISeriesApi,
  CandlestickData,
  Time,
} from "lightweight-charts";
import { Paper, Text } from "@mantine/core";
import type { Trade, CandleData } from "../types";

interface CandleChartProps {
  trades: Trade[];
}

export default function CandleChart({ trades }: CandleChartProps) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<"Candlestick"> | null>(null);
  const allCandlesRef = useRef<CandlestickData<Time>[]>([]);
  const isInitializedRef = useRef(false);
  const userInteractingRef = useRef(false);

  useEffect(() => {
    if (!chartContainerRef.current) return;

    // 차트 생성
    const chart = createChart(chartContainerRef.current, {
      width: chartContainerRef.current.clientWidth,
      height: 400,
      layout: {
        background: { color: "#ffffff" },
        textColor: "#333",
      },
      grid: {
        vertLines: { color: "#f0f0f0" },
        horzLines: { color: "#f0f0f0" },
      },
      timeScale: {
        timeVisible: true,
        secondsVisible: true,
        rightOffset: 0,
        barSpacing: 5,
      },
    });

    const candlestickSeries = chart.addCandlestickSeries({
      upColor: "#26a69a",
      downColor: "#ef5350",
      borderVisible: false,
      wickUpColor: "#26a69a",
      wickDownColor: "#ef5350",
    });

    chartRef.current = chart;
    seriesRef.current = candlestickSeries;

    // 사용자 상호작용 추적
    const timeScale = chart.timeScale();
    const handleVisibleRangeChange = () => {
      // 사용자가 스크롤/줌을 조작하면 플래그 설정
      userInteractingRef.current = true;
      // 일정 시간 후 플래그 해제 (자동 스크롤 재개)
      setTimeout(() => {
        userInteractingRef.current = false;
      }, 2000);
    };

    timeScale.subscribeVisibleTimeRangeChange(handleVisibleRangeChange);

    // 리사이즈 핸들러
    const handleResize = () => {
      if (chartContainerRef.current && chartRef.current) {
        chartRef.current.applyOptions({
          width: chartContainerRef.current.clientWidth,
        });
      }
    };

    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      chart.remove();
    };
  }, []);

  // 거래 데이터를 캔들 데이터로 변환하고 누적
  useEffect(() => {
    if (!seriesRef.current || trades.length === 0) return;

    const newCandles = convertTradesToCandles(trades);

    // 기존 캔들과 새 캔들을 병합 (중복 제거)
    const candleMap = new Map<number, CandlestickData<Time>>();

    // 기존 캔들 추가
    allCandlesRef.current.forEach((candle) => {
      const time =
        typeof candle.time === "number"
          ? candle.time
          : parseInt(candle.time as string);
      candleMap.set(time, candle);
    });

    // 새 캔들 추가/업데이트
    newCandles.forEach((candle) => {
      const time =
        typeof candle.time === "number"
          ? candle.time
          : parseInt(candle.time as string);
      const existing = candleMap.get(time);

      if (existing) {
        // 기존 캔들 업데이트 (high, low, close, volume 업데이트)
        candleMap.set(time, {
          ...existing,
          high: Math.max(existing.high, candle.high),
          low: Math.min(existing.low, candle.low),
          close: candle.close, // 최신 거래가 종가
        });
      } else {
        // 새 캔들 추가
        candleMap.set(time, candle);
      }
    });

    // 시간순으로 정렬
    const allCandles = Array.from(candleMap.values()).sort((a, b) => {
      const timeA =
        typeof a.time === "number" ? a.time : parseInt(a.time as string);
      const timeB =
        typeof b.time === "number" ? b.time : parseInt(b.time as string);
      return timeA - timeB;
    });

    // 연속성 유지 (이전 캔들의 종가를 다음 캔들의 시가로)
    for (let i = 1; i < allCandles.length; i++) {
      allCandles[i].open = allCandles[i - 1].close;
    }

    // 모든 데이터 설정
    allCandlesRef.current = allCandles;
    seriesRef.current.setData(allCandles);

    // 초기 로드 시에만 전체 범위 설정, 이후에는 사용자가 조작하지 않을 때만 자동 스크롤
    if (allCandles.length > 0 && chartRef.current) {
      if (!isInitializedRef.current) {
        // 초기 로드: 전체 데이터 범위 표시
        const firstTime =
          typeof allCandles[0].time === "number"
            ? allCandles[0].time
            : parseInt(String(allCandles[0].time));
        const lastTime =
          typeof allCandles[allCandles.length - 1].time === "number"
            ? allCandles[allCandles.length - 1].time
            : parseInt(String(allCandles[allCandles.length - 1].time));

        if (typeof firstTime === "number" && typeof lastTime === "number") {
          chartRef.current.timeScale().setVisibleRange({
            from: firstTime as Time,
            to: (lastTime + 1) as Time,
          });
          isInitializedRef.current = true;
        }
      }
      // 이후에는 데이터만 업데이트하고 사용자가 자유롭게 조작할 수 있도록 함
      // setVisibleRange를 호출하지 않아서 사용자의 스크롤/줌 위치가 유지됨
    }
  }, [trades]);

  return (
    <Paper p="md" withBorder shadow="sm" style={{ backgroundColor: "white" }}>
      <Text size="xl" fw={700} mb="md" style={{ color: "#1a1a1a" }}>
        캔들 차트
      </Text>
      <div
        ref={chartContainerRef}
        style={{
          width: "100%",
          height: "400px",
          borderRadius: "8px",
          overflow: "hidden",
          border: "1px solid #e0e0e0",
        }}
      />
    </Paper>
  );
}

// 거래 데이터를 캔들 데이터로 변환 (1초 단위)
function convertTradesToCandles(trades: Trade[]): CandlestickData<Time>[] {
  if (trades.length === 0) return [];

  // 시간별로 그룹화 (1초 단위)
  const candlesMap = new Map<number, CandleData>();

  trades.forEach((trade) => {
    const timestamp = new Date(trade.timestamp).getTime();
    const second = Math.floor(timestamp / 1000) * 1000; // 1초 단위로 반올림

    if (!candlesMap.has(second)) {
      candlesMap.set(second, {
        time: (second / 1000) as number, // Unix timestamp (초 단위)
        open: trade.price,
        high: trade.price,
        low: trade.price,
        close: trade.price,
        volume: trade.quantity,
      });
    } else {
      const candle = candlesMap.get(second)!;
      candle.high = Math.max(candle.high, trade.price);
      candle.low = Math.min(candle.low, trade.price);
      candle.close = trade.price; // 마지막 거래가 종가
      candle.volume += trade.quantity;
    }
  });

  // 시간순으로 정렬
  const candles = Array.from(candlesMap.values()).sort(
    (a, b) => a.time - b.time
  );

  // 첫 거래의 가격을 이전 캔들의 종가로 설정 (연속성 유지)
  for (let i = 1; i < candles.length; i++) {
    candles[i].open = candles[i - 1].close;
  }

  return candles.map((candle) => ({
    time: candle.time as Time,
    open: candle.open,
    high: candle.high,
    low: candle.low,
    close: candle.close,
  }));
}
