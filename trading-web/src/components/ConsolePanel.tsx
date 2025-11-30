import { Paper, Text, ScrollArea, Badge, Stack } from "@mantine/core";
import type { ConsoleLog } from "../types";

interface ConsolePanelProps {
  logs: ConsoleLog[];
}

export default function ConsolePanel({ logs }: ConsolePanelProps) {
  const getColor = (level: ConsoleLog["level"]) => {
    switch (level) {
      case "error":
        return "red";
      case "warn":
        return "yellow";
      case "success":
        return "green";
      default:
        return "blue";
    }
  };

  return (
    <Paper shadow="sm" p="md" withBorder h="100%">
      <Stack gap="xs" h="100%">
        <Text fw={600} size="lg">
          트레이딩봇 콘솔
        </Text>
        <ScrollArea h="100%" style={{ flex: 1 }}>
          <Stack gap="xs">
            {logs.length > 0 ? (
              logs.map((log, index) => (
                <div key={index} style={{ display: "flex", gap: "8px", alignItems: "flex-start" }}>
                  <Badge
                    size="sm"
                    variant="light"
                    color={getColor(log.level)}
                    style={{ minWidth: "60px" }}
                  >
                    {log.level.toUpperCase()}
                  </Badge>
                  <Text size="xs" c="dimmed" style={{ minWidth: "180px" }}>
                    {new Date(log.timestamp).toLocaleString("ko-KR")}
                  </Text>
                  <Text size="sm" style={{ flex: 1 }}>
                    {log.message}
                  </Text>
                </div>
              ))
            ) : (
              <Text c="dimmed" size="sm">
                콘솔 로그가 없습니다
              </Text>
            )}
          </Stack>
        </ScrollArea>
      </Stack>
    </Paper>
  );
}

