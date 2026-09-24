//! Time as the core sees it: milliseconds since the platform started.

pub use core::time::Duration;

/// Milliseconds since the platform started. Monotonic. A u64 never wraps in practice.
///
/// There is deliberately no `Instant - Instant` operator; use [`Instant::saturating_since`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(u64);

impl Instant {
    /// The latest representable instant. Schedules saturate here.
    pub const MAX: Instant = Instant(u64::MAX);

    #[must_use]
    pub const fn from_millis(ms: u64) -> Self {
        Instant(ms)
    }

    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }

    /// Time elapsed since `earlier`, or zero if `earlier` is later.
    #[must_use]
    pub const fn saturating_since(self, earlier: Instant) -> Duration {
        Duration::from_millis(self.0.saturating_sub(earlier.0))
    }
}

impl core::ops::Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, d: Duration) -> Instant {
        // Saturates rather than overflowing, so no timestamp can make `handle` panic.
        Instant(self.0.saturating_add(millis(d)))
    }
}

/// `d` in whole milliseconds, saturating at `u64::MAX`.
pub(crate) fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturating_since_is_zero_when_earlier_is_later() {
        let a = Instant::from_millis(10);
        let b = Instant::from_millis(20);
        assert_eq!(b.saturating_since(a), Duration::from_millis(10));
        assert_eq!(a.saturating_since(b), Duration::ZERO);
    }

    #[test]
    fn add_saturates() {
        let near_max = Instant::from_millis(u64::MAX - 1);
        assert_eq!(
            near_max + Duration::from_secs(1),
            Instant::from_millis(u64::MAX)
        );
        assert_eq!(near_max + Duration::MAX, Instant::from_millis(u64::MAX));
    }
}
