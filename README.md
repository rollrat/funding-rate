# Arbitrage-2

암호화폐 선물/현물 가격을 여러 거래소에서 모아 보여주고, 베이시스(선물-현물) 차익거래를 실험하기 위한 서버·CLI·웹 UI 세트입니다.

## 구성

- `server/` (Rust)
  - `crates/interface`: 거래소 공통 타입과 에러 정의.
  - `crates/exchanges`: Binance, Bybit, OKX, Bitget, Bithumb REST/WebSocket 클라이언트와 수수료·환율 조회 로직.
  - `crates/oracle`: 10초마다 선물/현물 시세와 USD/KRW·USDT/USD 환율을 수집해 `UnifiedSnapshot`으로 병합하고 HTTP로 제공합니다. 엔드포인트: `/health`, `/snapshots`, `/spot-snapshots`, `/unified-snapshots` (기본 포트 12090, CORS 허용).
  - `crates/trade`: 베이시스 차익거래 전략(`IntraBasisArbitrageStrategy`)과 자산/주문 탐색 도구 CLI. `run`, `explore-test`, `arbitrage-test`, `emergency-test` 명령을 제공합니다.
- `web/` (React + Vite + TypeScript + Mantine)
  - `/unified-snapshots` 응답을 10초 주기로 폴링해 거래소별 선물·현물 시세, 펀딩률, 거래량, 환율을 테이블로 표시합니다.

## 사전 준비

- Rust 1.76 이상(스테이블), Node.js 18+ 환경.
- 실제 거래/주문 테스트 시 거래소 API 키가 필요하며, `server/.env` 형식에 맞춰 설정합니다. (민감정보이므로 버전에 포함하지 마세요.)

## 빠른 실행 예시

- Oracle 서버 실행: `cd server && cargo run -p oracle` (12090 포트에서 스냅샷 제공)
- 차익거래 드라이런: `cd server && cargo run -p trade -- arbitrage-test`
- 웹 UI: `cd web && npm install && npm run dev -- --host` (혹은 빌드된 `dist/` 사용)

## 동작 개요

1. 컬렉터가 다중 거래소에서 선물/현물 시세와 환율을 수집합니다.
2. 종목·거래소 단위로 선물/현물 정보를 병합한 `UnifiedSnapshot`을 최신 타임스탬프와 함께 메모리에 저장합니다.
3. Axum 서버가 위 스냅샷을 JSON으로 노출하고, 웹 UI가 이를 폴링해 모니터링 테이블을 갱신합니다.
4. CLI 도구는 동일한 스냅샷·거래소 API를 활용해 베이시스 차익거래 전략을 시뮬레이션하거나(드라이런) 실제 주문 실행용 코드 베이스를 제공합니다.
