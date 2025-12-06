# Simulator-Web

Sim-Exchange 시뮬레이션 서버의 실시간 웹 인터페이스입니다.

## 기능

- **실시간 오더북**: 매수/매도 주문을 실시간으로 표시
- **체결 내역**: 최근 체결된 거래 내역을 테이블로 표시
- **캔들 차트**: 거래 데이터를 기반으로 한 1분 단위 캔들스틱 차트
- **주문 제출**: 지정가/시장가 주문을 직접 제출

## 사전 준비

- Node.js 18+ 환경
- Sim-Exchange 서버가 `http://localhost:3000`에서 실행 중이어야 합니다

## 설치 및 실행

```bash
cd simulator-web
npm install
npm run dev
```

웹 애플리케이션은 `http://localhost:3002`에서 실행됩니다.

## 프로젝트 구조

```
simulator-web/
├── src/
│   ├── components/
│   │   ├── OrderBook.tsx      # 오더북 컴포넌트
│   │   ├── TradeTable.tsx     # 체결 내역 테이블
│   │   ├── CandleChart.tsx    # 캔들 차트 (lightweight-charts 사용)
│   │   └── OrderForm.tsx      # 주문 제출 폼
│   ├── api.ts                 # API 클라이언트
│   ├── types.ts               # TypeScript 타입 정의
│   ├── App.tsx                # 메인 앱 컴포넌트
│   └── main.tsx               # 진입점
├── package.json
└── vite.config.ts
```

## 사용 기술

- **React 18**: UI 프레임워크
- **TypeScript**: 타입 안정성
- **Vite**: 빌드 도구
- **Mantine**: UI 컴포넌트 라이브러리
- **lightweight-charts**: TradingView 스타일의 차트 라이브러리
- **Tailwind CSS**: 유틸리티 CSS 프레임워크

## 데이터 갱신

- 오더북과 체결 내역은 500ms마다 자동으로 갱신됩니다 (시뮬레이션 서버의 주기와 동일)
- 주문 제출 후 즉시 데이터가 새로고침됩니다

