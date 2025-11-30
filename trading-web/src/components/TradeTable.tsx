import { Table, Badge, Text, ScrollArea } from "@mantine/core";
import type { TradeRecord } from "../types";

interface TradeTableProps {
  records: TradeRecord[];
  onRecordDoubleClick?: (record: TradeRecord) => void;
}

export default function TradeTable({
  records,
  onRecordDoubleClick,
}: TradeTableProps) {
  const rows = records.map((record) => (
    <Table.Tr
      key={record.id}
      onDoubleClick={() => onRecordDoubleClick?.(record)}
      style={{ cursor: onRecordDoubleClick ? "pointer" : undefined }}
    >
      <Table.Td>
        <Text size="sm">
          {new Date(record.executed_at).toLocaleString("ko-KR")}
        </Text>
      </Table.Td>
      <Table.Td>
        <Badge variant="light" color="blue">
          {record.exchange}
        </Badge>
      </Table.Td>
      <Table.Td>
        <Text fw={500}>{record.symbol}</Text>
      </Table.Td>
      <Table.Td>
        <Badge
          variant="light"
          color={record.market_type === "SPOT" ? "green" : "orange"}
        >
          {record.market_type}
        </Badge>
      </Table.Td>
      <Table.Td>
        <Badge variant="light" color={record.side === "BUY" ? "green" : "red"}>
          {record.side}
        </Badge>
      </Table.Td>
      <Table.Td>
        <Text size="sm">{record.trade_type}</Text>
      </Table.Td>
      <Table.Td>
        <Text size="sm">
          {record.executed_price
            ? record.executed_price.toLocaleString("ko-KR", {
                minimumFractionDigits: 2,
                maximumFractionDigits: 8,
              })
            : "N/A"}
        </Text>
      </Table.Td>
      <Table.Td>
        <Text size="sm">
          {record.quantity.toLocaleString("ko-KR", {
            minimumFractionDigits: 2,
            maximumFractionDigits: 8,
          })}
        </Text>
      </Table.Td>
      <Table.Td>
        {record.is_liquidation ? (
          <Badge color="red" variant="filled">
            청산
          </Badge>
        ) : (
          <Badge color="gray" variant="light">
            일반
          </Badge>
        )}
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <ScrollArea h="100%" type="scroll">
      <Table.ScrollContainer minWidth={800}>
        <Table striped highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>실행 시간</Table.Th>
              <Table.Th>거래소</Table.Th>
              <Table.Th>심볼</Table.Th>
              <Table.Th>마켓 타입</Table.Th>
              <Table.Th>방향</Table.Th>
              <Table.Th>거래 유형</Table.Th>
              <Table.Th>실행 가격</Table.Th>
              <Table.Th>수량</Table.Th>
              <Table.Th>상태</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {rows.length > 0 ? (
              rows
            ) : (
              <Table.Tr>
                <Table.Td colSpan={9}>
                  <Text c="dimmed" ta="center">
                    거래 기록이 없습니다
                  </Text>
                </Table.Td>
              </Table.Tr>
            )}
          </Table.Tbody>
        </Table>
      </Table.ScrollContainer>
    </ScrollArea>
  );
}
