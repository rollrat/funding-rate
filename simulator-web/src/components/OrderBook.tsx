import { Paper, Table, Text, Badge } from "@mantine/core";
import type { Order } from "../types";

interface OrderBookProps {
  bids: Order[];
  asks: Order[];
}

export default function OrderBook({ bids, asks }: OrderBookProps) {
  const MAX_DISPLAY = 5;

  const formatPrice = (price: number | null) => {
    if (price === null) return "Market";
    return price.toFixed(2);
  };

  const formatQuantity = (qty: number) => {
    return qty.toFixed(4);
  };

  // 매도 호가를 내림차순으로 정렬 (높은 가격부터, aggregate 없음)
  // 서버에서 오름차순으로 오므로 역순으로 정렬
  const sortedAsks = [...asks].sort((a, b) => {
    // null 값은 맨 뒤로
    if (a.price === null && b.price === null) return 0;
    if (a.price === null) return 1;
    if (b.price === null) return -1;

    const priceA = a.price;
    const priceB = b.price;
    return priceA - priceB; // 내림차순 (높은 가격부터)
  });

  // 매수 호가를 내림차순으로 정렬 (높은 가격부터, aggregate 없음)
  const sortedBids = [...bids].sort((a, b) => {
    const priceA = a.price ?? 0;
    const priceB = b.price ?? 0;
    return priceB - priceA; // 내림차순
  });

  // 최대 5개씩만 표시하고, 부족하면 null로 채움
  const displayAsks = sortedAsks.slice(0, MAX_DISPLAY).reverse();
  const displayBids = sortedBids.slice(0, MAX_DISPLAY);

  // 5개 미만이면 빈 행 추가
  const paddedAsks: (Order | null)[] = [
    ...Array(Math.max(0, MAX_DISPLAY - displayAsks.length)).fill(null),
    ...displayAsks,
  ];

  const paddedBids: (Order | null)[] = [...displayBids];
  while (paddedBids.length < MAX_DISPLAY) {
    paddedBids.push(null);
  }

  return (
    <Paper p="md" withBorder shadow="sm" style={{ backgroundColor: "white" }}>
      <Text size="xl" fw={700} mb="md" style={{ color: "#1a1a1a" }}>
        오더북
      </Text>
      <div style={{ overflow: "hidden" }}>
        <Table
          striped
          highlightOnHover
          withTableBorder
          withColumnBorders
          style={{ fontSize: "12px", tableLayout: "fixed", width: "100%" }}
        >
          <colgroup>
            <col style={{ width: "25%" }} />
            <col style={{ width: "30%" }} />
            <col style={{ width: "25%" }} />
            <col style={{ width: "20%" }} />
          </colgroup>
          <Table.Thead>
            <Table.Tr style={{ backgroundColor: "#f8f9fa" }}>
              <Table.Th
                style={{
                  fontWeight: 600,
                  textAlign: "left",
                  padding: "4px 8px",
                  width: "25%",
                }}
              >
                수량
              </Table.Th>
              <Table.Th
                style={{
                  fontWeight: 600,
                  textAlign: "center",
                  padding: "4px 8px",
                  width: "30%",
                }}
              >
                가격
              </Table.Th>
              <Table.Th
                style={{
                  fontWeight: 600,
                  textAlign: "right",
                  padding: "4px 8px",
                  width: "25%",
                }}
              >
                수량
              </Table.Th>
              <Table.Th
                style={{ fontWeight: 600, padding: "4px 8px", width: "20%" }}
              >
                타입
              </Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {/* 매도 주문 (내림차순 - 높은 가격부터, aggregate 없음) */}
            {paddedAsks.map((ask, index) => (
              <Table.Tr
                key={ask ? `ask-${ask.id}-${index}` : `ask-empty-${index}`}
                style={{ backgroundColor: "#fff5f5" }}
              >
                <Table.Td
                  style={{
                    textAlign: "left",
                    padding: "2px 8px",
                    width: "25%",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                >
                  <Text size="xs" c={ask ? undefined : "dimmed"}>
                    {ask ? formatQuantity(ask.quantity) : "0.0000"}
                  </Text>
                </Table.Td>
                <Table.Td
                  style={{
                    textAlign: "center",
                    padding: "2px 8px",
                    width: "30%",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                >
                  <Text
                    c={ask ? "red" : "dimmed"}
                    fw={ask ? 700 : 400}
                    size="xs"
                  >
                    {ask ? formatPrice(ask.price) : "0.00"}
                  </Text>
                </Table.Td>
                <Table.Td
                  style={{
                    textAlign: "right",
                    padding: "2px 8px",
                    width: "25%",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                >
                  <Text size="xs" style={{ opacity: 0 }}>
                    {ask ? formatQuantity(ask.quantity) : "0.0000"}
                  </Text>
                </Table.Td>
                <Table.Td
                  style={{
                    padding: "2px 8px",
                    width: "20%",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                >
                  {ask ? (
                    <Badge color="red" variant="light" size="xs">
                      {ask.order_type}
                    </Badge>
                  ) : (
                    <Badge variant="light" size="xs" style={{ opacity: 0 }}>
                      -
                    </Badge>
                  )}
                </Table.Td>
              </Table.Tr>
            ))}
            {/* 매수 주문 (내림차순 - 높은 가격부터, aggregate 없음) */}
            {paddedBids.map((bid, index) => (
              <Table.Tr
                key={bid ? `bid-${bid.id}-${index}` : `bid-empty-${index}`}
                style={{ backgroundColor: "#f0fdf4" }}
              >
                <Table.Td
                  style={{
                    textAlign: "left",
                    padding: "2px 8px",
                    width: "25%",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                >
                  <Text size="xs" style={{ opacity: 0 }}>
                    {bid ? formatQuantity(bid.quantity) : "0.0000"}
                  </Text>
                </Table.Td>
                <Table.Td
                  style={{
                    textAlign: "center",
                    padding: "2px 8px",
                    width: "30%",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                >
                  <Text
                    c={bid ? "green" : "dimmed"}
                    fw={bid ? 700 : 400}
                    size="xs"
                  >
                    {bid ? formatPrice(bid.price) : "0.00"}
                  </Text>
                </Table.Td>
                <Table.Td
                  style={{
                    textAlign: "right",
                    padding: "2px 8px",
                    width: "25%",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                >
                  <Text size="xs" c={bid ? undefined : "dimmed"}>
                    {bid ? formatQuantity(bid.quantity) : "0.0000"}
                  </Text>
                </Table.Td>
                <Table.Td
                  style={{
                    padding: "2px 8px",
                    width: "20%",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                  }}
                >
                  {bid ? (
                    <Badge color="green" variant="light" size="xs">
                      {bid.order_type}
                    </Badge>
                  ) : (
                    <Badge variant="light" size="xs" style={{ opacity: 0 }}>
                      -
                    </Badge>
                  )}
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </div>
    </Paper>
  );
}
