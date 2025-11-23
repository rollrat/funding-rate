# Arbitrage-2 Server (Rust)

암호화폐 거래소 간 가격 정보를 수집하고 arbitrage 전략을 실험·자동화하기 위한 러스트 워크스페이스입니다. Oracle 서비스가 여러 거래소의 선물·현물 시세를 모아 HTTP로 제공하고, Trade CLI가 이 데이터를 활용해 자산 조회나 아비트라지 전략 실행을 담당합니다.

## 프로젝트 구성

- `crates/interface`

  - 공용 데이터 모델과 에러 타입 집합. 거래소 식별자, 통화, 선물/현물 스냅샷, 환율 정보, 수수료/자산/호가창 구조체를 정의합니다.
  - 각 서비스와 클라이언트가 동일한 타입을 공유해 직렬화/역직렬화, HTTP 응답, 상태 저장에 사용됩니다.

- `crates/exchanges`

  - 각 거래소별 REST/WebSocket 클라이언트 모음. 표준화된 트레이트(`PerpExchange`, `SpotExchange`, `AssetExchange`, `OrderBookExchange`, `FeeExchange`)를 구현해 호출 측이 거래소별 차이를 신경 쓰지 않고 데이터를 수집할 수 있게 합니다.
  - 지원 거래소: Binance, Bybit, OKX, Bitget, Bithumb. 인증이 필요한 자산/주문·수수료 API 호출을 위해 `.env`의 키를 읽습니다.
  - 환율 유틸(`exchange_rate`)이 USD/KRW, USDT/USD, USDT/KRW를 주기적으로 조회해 스냅샷에 포함할 수 있게 합니다.

- `crates/oracle`

  - 백그라운드 수집기(`collector`)가 일정 주기(기본 10초)로 모든 거래소의 선물·현물 시세를 fetch→정렬→메모리에 적재합니다. 환율 정보도 함께 가져와 `UnifiedSnapshot`에 병합합니다.
  - Axum 기반 HTTP 서버(`server`)가 수집된 선물/현물/통합 스냅샷을 JSON으로 제공합니다. 단일 인스턴스로 동작하며, 클라이언트가 가벼운 API로 최신 시세를 가져갈 수 있도록 설계되었습니다.

- `crates/trade`
  - 실행형 CLI. 모드별 서브커맨드:
    - `explore-test`: Oracle에서 통합 스냅샷을 가져오거나, 거래소 인증 API로 자산 정보를 조회합니다(키 필요).
    - `arbitrage-test`: 아비트라지 전략 파라미터를 검증하는 드라이런.
    - `run`: 실제 아비트라지 자동화 자리를 위해 준비된 엔트리(현재 `todo!()` 남음).
  - `arbitrage` 모듈은 Binance 현물+선물을 활용한 아비트라지 전략(`BasisArbitrageStrategy`)을 구현하고, 포지션 상태를 `arb_state.json`으로 관리해 재시작 시 이어서 동작할 수 있게 합니다.

## 전략

- ExecutionPolicy: 주문 집행 정책 모음. 기본은 taker-taker이며, spot maker/futures taker, 양측 maker, maker 선행 후 taker, 기회형 maker, taker/maker TWAP, maker 그리드 등으로 주문 성격을 선택합니다.
- LegExecutionPolicy: 현물/선물 레그별로 시장가 taker, 공격적 리밋(taker 성향), 패시브 maker, post-only maker 중에서 지정합니다.

### 아비트라지 전략 (베이시스)

- 모드: carry (선물 프리미엄 시 현물 매수 + 선물 매도), reverse (현물 디스카운트 시 현물 매도 + 선물 매수), auto (조건에 따라 자동 선택).
- 진입/청산: 베이시스(bps) 기반 entry/exit 임계값. 노미널, 레버리지, 마진모드(교차/격리), 드라이런 여부를 파라미터로 조정합니다.
- 실행 흐름: Binance exchangeInfo 로드 → LOT_SIZE 기반 수량 조정 → 선물 레버리지·마진 설정 → 베이시스 계산 → 조건 충족 시 carry/reverse 진입·청산 → rb_state.json에 상태 기록(드라이런은 주문 미발행).

## 필수 요건

- Rust 1.76+ (stable 권장)
- 네트워크로 거래소 공개/인증 API 접근 가능해야 합니다.
- `.env` 또는 환경변수에 거래소 키를 설정하세요 (실제 키는 버전에 올리지 마세요).
  - `BINANCE_API_KEY`, `BINANCE_API_SECRET` (선물·현물 둘 다 사용)
  - `BITHUMB_API_KEY`, `BITHUMB_API_SECRET`
  - 그 외 공개 API는 키 없이 동작하지만, 자산 조회나 주문 관련 기능은 키가 필요합니다.

## 실행 방법

1. Oracle 서버 기동 (시세 수집 + HTTP 제공)

```bash
cargo run -p oracle
```

- 기본 포트: `12090`
- 엔드포인트:
  - `/health` : 상태 체크
  - `/snapshots` : 선물 스냅샷 목록
  - `/spot-snapshots` : 현물 스냅샷 목록
  - `/unified-snapshots` : 선물·현물·환율을 합친 스냅샷

2. Trade CLI 사용 예시

```bash
# Bithumb / Binance 자산 조회 (인증 키 필요)
cargo run -p trade -- explore-test

# 아비트라지 전략 파라미터 확인만 하는 드라이런
cargo run -p trade -- arbitrage-test
```

- `trade run` 커맨드는 아비트라지 전략 실행을 위한 자리이며 현재 `todo!()`로 구현이 남아 있습니다. 실제 자동 매매를 붙일 때 `BasisArbitrageStrategy::run_loop`를 호출하도록 확장하면 됩니다.

## 동작 흐름 개요

1. Oracle(`crates/oracle`)이 10초 간격으로 각 거래소의 선물/현물 시세를 가져와 정렬 후 메모리에 보관합니다. 동시에 USD/KRW, USDT/USD, USDT/KRW 환율을 조회합니다.
2. 수집된 데이터를 `/unified-snapshots` 등 HTTP 엔드포인트로 제공합니다.
3. Trade CLI(`crates/trade`)는 Oracle을 조회하거나 거래소 인증 API를 직접 호출해 자산/주문을 처리하고, 아비트라지 전략은 Binance 선물·현물 양쪽을 사용해 진입/청산을 결정합니다.

## 기타

- `build.sh`/`build.bat`, `Dockerfile`가 포함되어 있지만 현재 워크스페이스 구조와 완전히 맞지 않을 수 있으니 사용 전 경로를 검토하세요.
- 실계정 키가 담긴 `.env`는 절대 커밋하지 마세요.
