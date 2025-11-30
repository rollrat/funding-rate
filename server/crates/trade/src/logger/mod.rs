use std::{fs::OpenOptions, path::PathBuf};

use chrono::Local;
use tracing_appender::non_blocking;
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Tracing guards를 보관하는 구조체
/// 이 구조체가 drop되기 전까지 로깅이 계속 작동합니다
pub struct TracingGuards {
    _file: tracing_appender::non_blocking::WorkerGuard,
    _stdout: tracing_appender::non_blocking::WorkerGuard,
}

/// Tracing 초기화
/// 파일 로깅과 stdout 로깅을 모두 설정합니다
pub fn init_tracing() -> TracingGuards {
    // 1) 파일 appender
    let (file_writer, file_guard) = custom_daily_file_appender("logs", "trading");

    // 2) stdout도 non-blocking
    let (stdout_writer, stdout_guard) = non_blocking(std::io::stdout());

    // 3) EnvFilter
    let env_filter = EnvFilter::from_default_env().add_directive("info".parse().unwrap());

    // 4) 레이어 조립
    // 파일 로깅: INFO 레벨 이상만 기록
    let file_filter = EnvFilter::new("info");

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(file_writer)
                .with_filter(file_filter),
        )
        .with(fmt::layer().with_writer(stdout_writer).with_ansi(true))
        .init();

    // guards를 리턴해서 main에서 들고 있게 만들기
    TracingGuards {
        _file: file_guard,
        _stdout: stdout_guard,
    }
}

/// 날짜별 로그 파일 생성
/// `logs/trading.2025-11-29.log` 형식으로 파일을 생성합니다
fn custom_daily_file_appender(
    base_dir: &str,
    prefix: &str,
) -> (
    non_blocking::NonBlocking,
    tracing_appender::non_blocking::WorkerGuard,
) {
    // 날짜 문자열 생성: 2025-11-29
    let date = Local::now().format("%Y-%m-%d").to_string();

    // 최종 파일 이름: trading.2025-11-29.log
    let filename = format!("{prefix}.{date}.log");

    // logs/trading.2025-11-29.log
    let mut path = PathBuf::from(base_dir);
    path.push(filename);

    // 파일 오픈
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("Failed to open custom log file");

    non_blocking(file)
}
