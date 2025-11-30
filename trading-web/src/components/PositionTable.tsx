import { Table, Badge, Text, ScrollArea } from "@mantine/core";
import type { PositionRecord } from "../types";

interface PositionTableProps {
  records: PositionRecord[];
  selectedPositionId: number | null;
  onPositionClick: (positionId: number) => void;
}

export default function PositionTable({
  records,
  selectedPositionId,
  onPositionClick,
}: PositionTableProps) {
  const rows = records.map((record) => (
    <Table.Tr
      key={record.id}
      onClick={() => onPositionClick(record.id)}
      style={{
        cursor: "pointer",
        backgroundColor:
          selectedPositionId === record.id
            ? "var(--mantine-color-blue-light)"
            : undefined,
      }}
    >
      <Table.Td>
        <Text size="sm">
          {new Date(record.executed_at).toLocaleString("ko-KR")}
        </Text>
      </Table.Td>
      <Table.Td>
        <Badge variant="light" color="purple">
          {record.bot_name}
        </Badge>
      </Table.Td>
      <Table.Td>
        <Badge
          variant="light"
          color={record.carry === "CARRY" ? "blue" : "orange"}
        >
          {record.carry}
        </Badge>
      </Table.Td>
      <Table.Td>
        <Badge
          variant="light"
          color={record.action === "OPEN" ? "green" : "red"}
        >
          {record.action}
        </Badge>
      </Table.Td>
      <Table.Td>
        <Text fw={500}>{record.symbol}</Text>
      </Table.Td>
      <Table.Td>
        <Text size="sm">
          {record.spot_price.toLocaleString("ko-KR", {
            minimumFractionDigits: 2,
            maximumFractionDigits: 8,
          })}
        </Text>
      </Table.Td>
      <Table.Td>
        <Text size="sm">
          {record.futures_mark.toLocaleString("ko-KR", {
            minimumFractionDigits: 2,
            maximumFractionDigits: 8,
          })}
        </Text>
      </Table.Td>
      <Table.Td>
        <Badge variant="light" color="cyan">
          {record.buy_exchange}
        </Badge>
      </Table.Td>
      <Table.Td>
        <Badge variant="light" color="pink">
          {record.sell_exchange}
        </Badge>
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
              <Table.Th>봇 이름</Table.Th>
              <Table.Th>방향</Table.Th>
              <Table.Th>액션</Table.Th>
              <Table.Th>심볼</Table.Th>
              <Table.Th>스팟 가격</Table.Th>
              <Table.Th>선물 마크</Table.Th>
              <Table.Th>매수 거래소</Table.Th>
              <Table.Th>매도 거래소</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {rows.length > 0 ? (
              rows
            ) : (
              <Table.Tr>
                <Table.Td colSpan={9}>
                  <Text c="dimmed" ta="center">
                    포지션 기록이 없습니다
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
