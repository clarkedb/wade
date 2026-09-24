//! The desktop clock: milliseconds since startup, optionally sped up by `--time-scale`.

use std::time::Duration;

use wade_core::Instant;

pub struct Clock {
    start: std::time::Instant,
    scale: f64,
}

impl Clock {
    pub fn new(scale: f64) -> Self {
        assert!(
            scale.is_finite() && scale > 0.0,
            "--time-scale must be positive"
        );
        Clock {
            start: std::time::Instant::now(),
            scale,
        }
    }

    /// The current core time.
    pub fn now(&self) -> Instant {
        let ms = self.start.elapsed().as_secs_f64() * 1_000.0 * self.scale;
        Instant::from_millis(ms as u64)
    }

    /// Real time until the core clock reaches `t`, or zero if it already has.
    pub fn real_until(&self, t: Instant) -> Duration {
        let core = t.saturating_since(self.now());
        Duration::try_from_secs_f64(core.as_secs_f64() / self.scale).unwrap_or(Duration::MAX)
    }
}
