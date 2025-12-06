import { useState } from "react";
import {
  Paper,
  Text,
  Stack,
  Select,
  NumberInput,
  Button,
  Alert,
} from "@mantine/core";
import { IconAlertCircle } from "@tabler/icons-react";
import { submitOrder } from "../api";
import type { OrderRequest } from "../types";

interface OrderFormProps {
  onOrderSubmitted?: () => void;
}

export default function OrderForm({ onOrderSubmitted }: OrderFormProps) {
  const [side, setSide] = useState<"Buy" | "Sell">("Buy");
  const [orderType, setOrderType] = useState<"Limit" | "Market">("Limit");
  const [price, setPrice] = useState<number | "">(100);
  const [quantity, setQuantity] = useState<number | "">(1);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const handleSubmit = async () => {
    if (quantity === "" || quantity <= 0) {
      setError("수량을 입력해주세요");
      return;
    }

    if (orderType === "Limit" && (price === "" || price <= 0)) {
      setError("가격을 입력해주세요");
      return;
    }

    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      const order: OrderRequest = {
        side,
        order_type: orderType,
        quantity: quantity as number,
        ...(orderType === "Limit" && { price: price as number }),
      };

      const response = await submitOrder(order);
      setSuccess(
        `주문 제출 성공! ID: ${response.id}, 상태: ${response.status}, 체결: ${response.trades.length}건`
      );

      // 폼 초기화하지 않음 - 사용자가 같은 값으로 연속 주문할 수 있도록
      // 필요시 사용자가 수동으로 변경 가능

      if (onOrderSubmitted) {
        onOrderSubmitted();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "주문 제출 실패");
    } finally {
      setLoading(false);
    }
  };

  return (
    <Paper p="md" withBorder shadow="sm" style={{ backgroundColor: "white" }}>
      <Text size="xl" fw={700} mb="md" style={{ color: "#1a1a1a" }}>
        주문 제출
      </Text>
      <Stack gap="md">
        <Select
          label="매수/매도"
          value={side}
          onChange={(value) => setSide(value as "Buy" | "Sell")}
          data={[
            { value: "Buy", label: "매수" },
            { value: "Sell", label: "매도" },
          ]}
        />

        <Select
          label="주문 유형"
          value={orderType}
          onChange={(value) => setOrderType(value as "Limit" | "Market")}
          data={[
            { value: "Limit", label: "지정가" },
            { value: "Market", label: "시장가" },
          ]}
        />

        {orderType === "Limit" && (
          <NumberInput
            label="가격"
            value={price}
            onChange={setPrice}
            min={0}
            step={0.01}
            precision={2}
            required
          />
        )}

        <NumberInput
          label="수량"
          value={quantity}
          onChange={setQuantity}
          min={0}
          step={0.01}
          precision={4}
          required
        />

        {error && (
          <Alert icon={<IconAlertCircle size={16} />} title="오류" color="red">
            {error}
          </Alert>
        )}

        {success && (
          <Alert title="성공" color="green">
            {success}
          </Alert>
        )}

        <Button
          onClick={handleSubmit}
          loading={loading}
          fullWidth
          size="md"
          style={{ marginTop: "8px" }}
        >
          주문 제출
        </Button>
      </Stack>
    </Paper>
  );
}
