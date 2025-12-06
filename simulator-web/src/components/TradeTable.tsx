import { Paper, Table, Text, Badge, ScrollArea } from '@mantine/core';
import type { Trade } from '../types';

interface TradeTableProps {
  trades: Trade[];
}

export default function TradeTable({ trades }: TradeTableProps) {
  // 최신 거래부터 표시 (전체)
  const displayTrades = [...trades].reverse();

  const formatTime = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString('ko-KR', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      fractionalSecondDigits: 3,
    });
  };

  const formatPrice = (price: number) => {
    return price.toFixed(2);
  };

  const formatQuantity = (qty: number) => {
    return qty.toFixed(4);
  };

  return (
    <Paper p="md" withBorder shadow="sm" style={{ backgroundColor: 'white' }}>
      <Text size="xl" fw={700} mb="md" style={{ color: '#1a1a1a' }}>
        체결 내역
      </Text>
      <ScrollArea h={400} type="scroll">
        <Table 
          striped 
          highlightOnHover 
          withTableBorder 
          withColumnBorders
          style={{ fontSize: '14px' }}
        >
          <Table.Thead>
            <Table.Tr style={{ backgroundColor: '#f8f9fa' }}>
              <Table.Th style={{ fontWeight: 600 }}>시간</Table.Th>
              <Table.Th style={{ fontWeight: 600 }}>가격</Table.Th>
              <Table.Th style={{ fontWeight: 600 }}>수량</Table.Th>
              <Table.Th style={{ fontWeight: 600 }}>매수/매도</Table.Th>
              <Table.Th style={{ fontWeight: 600 }}>거래금액</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {displayTrades.length === 0 ? (
              <Table.Tr>
                <Table.Td colSpan={5}>
                  <Text ta="center" c="dimmed" py="xl">
                    체결 내역이 없습니다
                  </Text>
                </Table.Td>
              </Table.Tr>
            ) : (
              displayTrades.map((trade, index) => (
                <Table.Tr key={`${trade.timestamp}-${index}`}>
                  <Table.Td>
                    <Text size="sm" c="dimmed">
                      {formatTime(trade.timestamp)}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Text fw={700} size="sm" c="blue">
                      {formatPrice(trade.price)}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="sm">{formatQuantity(trade.quantity)}</Text>
                  </Table.Td>
                  <Table.Td>
                    <Badge 
                      variant="light" 
                      color={trade.side === 'Buy' ? 'green' : 'red'} 
                      size="sm"
                      style={{ fontWeight: 600 }}
                    >
                      {trade.side === 'Buy' ? '매수' : '매도'}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    <Badge variant="light" color="blue" size="sm">
                      {(trade.price * trade.quantity).toFixed(2)}
                    </Badge>
                  </Table.Td>
                </Table.Tr>
              ))
            )}
          </Table.Tbody>
        </Table>
      </ScrollArea>
    </Paper>
  );
}

