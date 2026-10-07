//! Measurements for the CYD port's done criteria (docs/roadmap.md#cyd-port).
//! Built only with the `measure` feature; logs a summary every 10 s and the
//! uptime every minute, for comparing against the host's clock.

use embassy_time::{Duration, Instant, Ticker};
use log::info;

const REPORT_EVERY: Duration = Duration::from_secs(10);
/// Frames closer together than this are one animation, for its frame rate.
const ANIMATING: Duration = Duration::from_millis(100);

pub struct Stats {
    since: Instant,
    last_frame: Option<Instant>,
    frames: u32,
    show_total: Duration,
    show_max: Duration,
    animation_frames: u32,
    animation_total: Duration,
    animation_max: Duration,
    touches: u32,
    touch_total: Duration,
    touch_max: Duration,
    deadlines: u32,
    late_total: Duration,
    late_max: Duration,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            since: Instant::now(),
            last_frame: None,
            frames: 0,
            show_total: Duration::MIN,
            show_max: Duration::MIN,
            animation_frames: 0,
            animation_total: Duration::MIN,
            animation_max: Duration::MIN,
            touches: 0,
            touch_total: Duration::MIN,
            touch_max: Duration::MIN,
            deadlines: 0,
            late_total: Duration::MIN,
            late_max: Duration::MIN,
        }
    }

    /// A deadline the core asked for at `due` was handled at `handled`.
    pub fn deadline(&mut self, due: Instant, handled: Instant) {
        let late = handled.saturating_duration_since(due);
        self.deadlines += 1;
        self.late_total += late;
        self.late_max = self.late_max.max(late);
    }

    /// A frame drawn from `started` to `finished`. `touch` is when the touch
    /// that caused it was sampled, if one did.
    pub fn frame(&mut self, started: Instant, finished: Instant, touch: Option<Instant>) {
        let show = finished - started;
        self.frames += 1;
        self.show_total += show;
        self.show_max = self.show_max.max(show);
        if let Some(last) = self.last_frame.replace(started) {
            let gap = started - last;
            if gap < ANIMATING {
                self.animation_frames += 1;
                self.animation_total += gap;
                self.animation_max = self.animation_max.max(gap);
            }
        }
        if let Some(at) = touch {
            let latency = finished.saturating_duration_since(at);
            self.touches += 1;
            self.touch_total += latency;
            self.touch_max = self.touch_max.max(latency);
        }
        if Instant::now() - self.since >= REPORT_EVERY {
            self.report();
            *self = Stats {
                last_frame: self.last_frame,
                ..Stats::new()
            };
        }
    }

    fn report(&self) {
        let average = |total: Duration, n: u32| total.as_micros() / u64::from(n.max(1));
        info!(
            "frames {}: show avg {} us, max {} us",
            self.frames,
            average(self.show_total, self.frames),
            self.show_max.as_micros()
        );
        if self.animation_frames > 0 {
            let gap = average(self.animation_total, self.animation_frames);
            info!(
                "animating: {} frames, {} fps average, worst gap {} ms",
                self.animation_frames,
                1_000_000 / gap.max(1),
                self.animation_max.as_millis()
            );
        }
        if self.deadlines > 0 {
            info!(
                "deadlines: {}, late avg {} us, max {} us",
                self.deadlines,
                average(self.late_total, self.deadlines),
                self.late_max.as_micros()
            );
        }
        if self.touches > 0 {
            info!(
                "touch to screen: {} touches, avg {} ms, max {} ms",
                self.touches,
                average(self.touch_total, self.touches) / 1000,
                self.touch_max.as_millis()
            );
        }
    }
}

/// Logs the uptime every minute. Each line's arrival time on the host, against
/// the uptime it reports, shows how far the board's clock drifts.
#[embassy_executor::task]
pub async fn uptime() {
    let mut ticker = Ticker::every(Duration::from_secs(60));
    loop {
        ticker.next().await;
        info!("uptime {} ms", Instant::now().as_millis());
    }
}
