use rand::Rng;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regime {
    Calm, // 저변동
    Normal,
    HighVol,    // 고변동
    FlashCrash, // 단기 폭락
    FlashPump,  // 단기 폭등
    WhaleAccum, // 세력 매집
    WhaleDump,  // 세력 투매
}

pub struct RegimeState {
    pub current: Regime,
    since: Instant,
}

impl RegimeState {
    pub fn new() -> Self {
        Self {
            current: Regime::Normal,
            since: Instant::now(),
        }
    }

    pub fn step<R: Rng>(&mut self, rng: &mut R) {
        let elapsed = self.since.elapsed();
        // let elapsed_secs = elapsed.as_secs_f64();
        let elapsed_secs = elapsed.as_millis() as f64 * 100.0;

        match self.current {
            Regime::Calm | Regime::Normal => {
                // Calm/Normal에서 일정 시간(30초) 이상 지나면 Low prob로 HighVol/WhaleAccum/WhaleDump로 전환
                if elapsed_secs > 30.0 {
                    let roll = rng.gen_range(0.0..1.0);
                    if roll < 0.15 {
                        // 15% 확률로 HighVol로 전환
                        self.current = Regime::HighVol;
                        self.since = Instant::now();
                    } else if roll < 0.25 {
                        // 10% 확률로 WhaleAccum으로 전환
                        self.current = Regime::WhaleAccum;
                        self.since = Instant::now();
                    } else if roll < 0.35 {
                        // 10% 확률로 WhaleDump로 전환
                        self.current = Regime::WhaleDump;
                        self.since = Instant::now();
                    }
                }
            }
            Regime::HighVol => {
                // HighVol은 20초 이상 지속 후 Normal로 복귀
                if elapsed_secs > 20.0 {
                    let roll = rng.gen_range(0.0..1.0);
                    if roll < 0.3 {
                        // 30% 확률로 FlashCrash로 전환
                        self.current = Regime::FlashCrash;
                        self.since = Instant::now();
                    } else if roll < 0.6 {
                        // 30% 확률로 FlashPump로 전환
                        self.current = Regime::FlashPump;
                        self.since = Instant::now();
                    } else {
                        // 40% 확률로 Normal로 복귀
                        self.current = Regime::Normal;
                        self.since = Instant::now();
                    }
                }
            }
            Regime::WhaleAccum => {
                // WhaleAccum은 60초 정도 진행 후 FlashPump로 전환
                if elapsed_secs > 60.0 {
                    self.current = Regime::FlashPump;
                    self.since = Instant::now();
                }
            }
            Regime::WhaleDump => {
                // WhaleDump는 60초 정도 진행 후 FlashCrash로 전환
                if elapsed_secs > 60.0 {
                    self.current = Regime::FlashCrash;
                    self.since = Instant::now();
                }
            }
            Regime::FlashCrash | Regime::FlashPump => {
                // FlashCrash/FlashPump는 10초 정도 지나면 Normal로 복귀
                if elapsed_secs > 10.0 {
                    self.current = Regime::Normal;
                    self.since = Instant::now();
                }
            }
        }
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.since.elapsed().as_secs_f64()
    }
}
