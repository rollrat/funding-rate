import { useState, useEffect } from "react";
import {
  Container,
  Title,
  Grid,
  Loader,
  Alert,
  Group,
  Badge,
  Paper,
} from "@mantine/core";
import { IconAlertCircle } from "@tabler/icons-react";
import OrderBook from "./components/OrderBook";
import TradeTable from "./components/TradeTable";
import CandleChart from "./components/CandleChart";
import OrderForm from "./components/OrderForm";
import { useWebSocket } from "./hooks/useWebSocket";

function App() {
  // WebSocket을 통한 실시간 데이터 수신
  const wsUrl = (() => {
    // 개발 환경에서는 프록시를 통해 연결
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    // 개발 환경에서는 Vite 프록시를 통해 연결
    if (
      window.location.port === "3002" ||
      window.location.hostname === "localhost"
    ) {
      return `${protocol}//${window.location.host}/ws`;
    } else {
      const host = window.location.hostname;
      const port = "3000";
      return `${protocol}//${host}:${port}/ws`;
    }
  })();

  const { orderBook, trades, connected, error: wsError } = useWebSocket(wsUrl);
  const [loading, setLoading] = useState(true);

  // WebSocket 연결 상태에 따라 로딩 상태 업데이트
  useEffect(() => {
    if (connected && orderBook) {
      setLoading(false);
    }
  }, [connected, orderBook]);

  const handleOrderSubmitted = () => {
    // 주문 제출은 REST API 사용 (WebSocket은 읽기 전용)
    // 주문 제출 후 서버가 자동으로 WebSocket으로 업데이트를 보냄
  };

  const displayError = wsError;

  if (loading && !orderBook) {
    return (
      <Container size="xl" py="xl">
        <Group justify="center" mt="xl">
          <Loader size="lg" />
        </Group>
      </Container>
    );
  }

  if (displayError && !orderBook) {
    return (
      <Container size="xl" py="xl">
        <Alert icon={<IconAlertCircle size={16} />} title="오류" color="red">
          {displayError}
        </Alert>
      </Container>
    );
  }

  const bestBid = orderBook?.bids[0]?.price ?? null;
  const bestAsk = orderBook?.asks[0]?.price ?? null;
  const spread = bestBid && bestAsk ? bestAsk - bestBid : null;
  const spreadPercent =
    spread && bestBid ? ((spread / bestBid) * 100).toFixed(4) : null;

  return (
    <div
      style={{
        minHeight: "100vh",
        backgroundColor: "#f5f5f5",
        padding: 0,
        margin: 0,
      }}
    >
      <Container size="xl" py="xl" style={{ maxWidth: "1400px" }}>
        <Paper
          p="lg"
          mb="xl"
          withBorder
          shadow="sm"
          style={{
            backgroundColor: "#ffffff",
            border: "1px solid #e0e0e0",
            borderRadius: "8px",
          }}
        >
          <Group
            justify="space-between"
            align="center"
            wrap="wrap"
            gap="md"
            style={{ width: "100%" }}
          >
            <Title
              order={1}
              style={{
                margin: 0,
                fontSize: "28px",
                fontWeight: 700,
                color: "#1a1a1a",
              }}
            >
              Sim-Exchange 실시간 거래소
            </Title>
            <Group gap="md">
              <Badge color={connected ? "green" : "red"} variant="light">
                {connected ? "연결됨" : "연결 끊김"}
              </Badge>
              {bestBid && (
                <Badge
                  color="green"
                  size="lg"
                  variant="filled"
                  style={{
                    fontSize: "14px",
                    padding: "8px 16px",
                    fontWeight: 600,
                    backgroundColor: "#22c55e",
                  }}
                >
                  매수: {bestBid.toFixed(2)}
                </Badge>
              )}
              {bestAsk && (
                <Badge
                  color="red"
                  size="lg"
                  variant="filled"
                  style={{
                    fontSize: "14px",
                    padding: "8px 16px",
                    fontWeight: 600,
                    backgroundColor: "#ef4444",
                  }}
                >
                  매도: {bestAsk.toFixed(2)}
                </Badge>
              )}
              {spread && spreadPercent && (
                <Badge
                  color="gray"
                  size="lg"
                  variant="light"
                  style={{
                    fontSize: "14px",
                    padding: "8px 16px",
                    fontWeight: 600,
                  }}
                >
                  스프레드: {spread.toFixed(2)} ({spreadPercent}%)
                </Badge>
              )}
            </Group>
          </Group>
        </Paper>

        <Grid gutter="md">
          {/* 캔들 차트 - 맨 위에 크게 */}
          <Grid.Col span={12}>
            <CandleChart trades={trades} />
          </Grid.Col>

          {/* 오더북과 주문 제출 - 나란히 */}
          <Grid.Col span={{ base: 12, md: 8 }}>
            {orderBook ? (
              <OrderBook bids={orderBook.bids} asks={orderBook.asks} />
            ) : (
              <Paper p="md" withBorder>
                <Loader size="md" />
              </Paper>
            )}
          </Grid.Col>
          <Grid.Col span={{ base: 12, md: 4 }}>
            <OrderForm onOrderSubmitted={handleOrderSubmitted} />
          </Grid.Col>

          {/* 체결 내역 - 오더북 밑에 */}
          <Grid.Col span={12}>
            <TradeTable trades={trades} />
          </Grid.Col>
        </Grid>
      </Container>
    </div>
  );
}

export default App;
