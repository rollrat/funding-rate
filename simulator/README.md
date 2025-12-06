# sim-exchange: Rust Trading Simulation Server

sim-exchange는 Rust 기반의 거래 시뮬레이션 서버로, "가상 거래소" 역할을 합니다. 거래 전략 개발자가 주문을 제출하고 실시간으로 매칭 및 거래 체결을 관찰할 수 있습니다. 서버는 비동기 Rust를 사용하며 Tokio와 Axum을 사용한 HTTP REST API를 제공합니다.

## 프로젝트 구조

```
simulator/
|--- Cargo.toml
|--- src/
    |--- main.rs              # 애플리케이션 진입점: 서버 설정 및 시뮬레이션 루프
    |--- domain/
    |   |--- mod.rs           # 도메인 모듈 (order, trade, snapshot 재내보내기)
    |   |--- order.rs         # 주문 타입 정의 (Order, OrderSide, OrderType)
    |   |--- trade.rs         # 거래 기록 정의
    |   |--- snapshot.rs      # 시장 스냅샷 정의 (주문 생성용)
    |--- engine/
    |   |--- mod.rs           # 엔진 모듈 (MatchingEngine, EngineError 재내보내기)
    |   |--- matching_engine.rs # 매칭 엔진: 오더북 및 매칭 로직
    |--- market/
    |   |--- mod.rs           # 시장 모듈 (트레이트 및 복합 플로우 오케스트레이터)
    |   |--- noise_trader.rs  # NoiseTrader: 랜덤 주문 플로우 생성기
    |   |--- passive_mm.rs    # PassiveMM: 패시브 마켓메이커 (선택적, 스텁/데모)
    |   |--- spike_generator.rs # SpikeGenerator: 가끔 큰 주문 생성기
    |   |--- composite.rs     # CompositeFlow: 여러 OrderFlowSource 구현 결합
    |--- gateway.rs           # HTTP REST API 핸들러 (Gateway)
```

## 의존성

주요 의존성:

- `tokio`: 비동기 런타임
- `axum`: 웹 프레임워크
- `serde`, `serde_json`: JSON 직렬화
- `uuid`: 고유 ID 생성
- `chrono`: 타임스탬프
- `rand`: 랜덤 주문 생성
- `thiserror`: 에러 정의

## 실행 방법

```bash
cd simulator
cargo run
```

서버는 기본적으로 `http://localhost:3000`에서 실행됩니다.

## API 엔드포인트

### GET /orderbook

현재 오더북(매수/매도 주문 목록)을 JSON으로 반환합니다.

**응답 예시:**

```json
{
  "bids": [
    {
      "id": "...",
      "side": "Buy",
      "order_type": "Limit",
      "price": 100.5,
      "quantity": 10.0,
      "timestamp": "2024-01-01T00:00:00Z"
    }
  ],
  "asks": [...]
}
```

### GET /trades

최근 거래 기록을 JSON 배열로 반환합니다.

**응답 예시:**

```json
[
  {
    "price": 100.5,
    "quantity": 5.0,
    "timestamp": "2024-01-01T00:00:00Z"
  }
]
```

### POST /order

새로운 주문을 제출합니다.

**요청 예시:**

```json
{
  "side": "Buy",
  "order_type": "Limit",
  "price": 101.5,
  "quantity": 10.0
}
```

**응답 예시:**

```json
{
  "id": "...",
  "status": "Filled",
  "trades": [
    {
      "price": 101.5,
      "quantity": 10.0,
      "timestamp": "2024-01-01T00:00:00Z"
    }
  ]
}
```

**주문 상태:**

- `"Open"`: 주문이 오더북에 남아있음 (미체결)
- `"Filled"`: 주문이 완전히 체결됨
- `"PartiallyFilled"`: 주문이 부분적으로 체결됨
- `"NotFilled"`: 시장가 주문이 유동성 부족으로 체결되지 않음

## 동작 원리

1. **시뮬레이션 루프**: 백그라운드 태스크가 500ms마다 실행되어:

   - 현재 시장 스냅샷을 가져옵니다
   - 여러 주문 생성 소스(NoiseTrader, PassiveMM, SpikeGenerator)에서 주문을 생성합니다
   - 생성된 주문을 매칭 엔진에 제출합니다

2. **매칭 엔진**:

   - 오더북을 관리하고 (매수/매도 주문을 가격별로 정렬)
   - 새로운 주문이 들어오면 반대편 오더북과 매칭을 시도합니다
   - 매칭되면 거래를 생성하고 기록합니다

3. **REST API**:
   - 클라이언트가 오더북 조회, 거래 조회, 주문 제출을 할 수 있습니다

## 확장 가능성

이 설계는 모듈화되어 있어 다음과 같이 확장할 수 있습니다:

- WebSocket 스트림으로 오더북 업데이트 및 거래 브로드캐스트
- 여러 거래 쌍 지원 (여러 MatchingEngine 인스턴스 관리)
- 리스크 엔진 통합으로 더 복잡한 주문 처리

현재 구현은 단일 상품 거래 시뮬레이션 환경을 REST 인터페이스로 제공합니다.
