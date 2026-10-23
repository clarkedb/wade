//! How pleased with himself Wade has been lately (docs/character.md#mood).

use crate::time::{Duration, Instant, millis};

/// The most points his mood holds.
pub const MOOD_MAX: u32 = 100;
/// His mood loses a point every `MOOD_DECAY`, so a full one fades in 10 min.
pub const MOOD_DECAY: Duration = Duration::from_secs(6);
/// Points from a tap on him while awake.
pub const MOOD_TAP: i32 = 8;
/// Points from dismissing a finished timer he takes credit for.
pub const MOOD_SUCCESS: i32 = 30;
/// Points lost when the timer catches him napping on the job.
pub const MOOD_CAUGHT: i32 = -40;

/// His mood in bands, from what its points are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MoodLevel {
    #[default]
    Plain,
    Pleased,
}

impl MoodLevel {
    /// The fewest points for this level.
    const fn floor(self) -> u32 {
        match self {
            MoodLevel::Plain => 0,
            MoodLevel::Pleased => 30,
        }
    }

    const fn of(points: u32) -> MoodLevel {
        if points >= MoodLevel::Pleased.floor() {
            MoodLevel::Pleased
        } else {
            MoodLevel::Plain
        }
    }
}

/// Points that fade one at a time, counted from when they last changed, so
/// the mood at any instant follows from this state alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Mood {
    points: u32,
    /// When `points` was set; a point fades every `MOOD_DECAY` after it.
    at: Instant,
}

impl Mood {
    pub const fn new(now: Instant) -> Mood {
        Mood { points: 0, at: now }
    }

    fn faded(&self, now: Instant) -> u64 {
        millis(now.saturating_since(self.at)) / millis(MOOD_DECAY)
    }

    pub fn points(&self, now: Instant) -> u32 {
        let faded = u32::try_from(self.faded(now)).unwrap_or(u32::MAX);
        self.points.saturating_sub(faded)
    }

    pub fn level(&self, now: Instant) -> MoodLevel {
        MoodLevel::of(self.points(now))
    }

    /// Add `delta` points at `now`, within 0 … `MOOD_MAX`. The fade keeps its
    /// rhythm: the next point goes when it would have.
    pub fn add(&mut self, now: Instant, delta: i32) {
        let points = self.points(now).saturating_add_signed(delta).min(MOOD_MAX);
        let into_step = millis(now.saturating_since(self.at)) % millis(MOOD_DECAY);
        self.points = points;
        self.at = Instant::from_millis(now.as_millis() - into_step);
    }

    /// When the mood next falls to a lower level, if it can.
    pub fn next_drop(&self, now: Instant) -> Option<Instant> {
        let level = self.level(now);
        if level == MoodLevel::Plain {
            return None;
        }
        let steps = u64::from(self.points - level.floor() + 1);
        Some(self.at + Duration::from_millis(steps * millis(MOOD_DECAY)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(ms: u64) -> Instant {
        Instant::from_millis(ms)
    }

    #[test]
    fn points_fade_one_per_step() {
        let mut mood = Mood::new(ms(0));
        mood.add(ms(0), 35);
        assert_eq!(mood.level(ms(0)), MoodLevel::Pleased);
        assert_eq!(mood.points(ms(5_999)), 35);
        assert_eq!(mood.points(ms(6_000)), 34);
        assert_eq!(mood.next_drop(ms(0)), Some(ms(36_000)));
        assert_eq!(mood.level(ms(35_999)), MoodLevel::Pleased);
        assert_eq!(mood.level(ms(36_000)), MoodLevel::Plain);
        assert_eq!(mood.next_drop(ms(36_000)), None);
    }

    #[test]
    fn adding_keeps_the_fade_rhythm() {
        let mut mood = Mood::new(ms(0));
        mood.add(ms(0), 10);
        mood.add(ms(9_000), 5);
        // One point faded at 6 s; the next still goes at 12 s.
        assert_eq!(mood.points(ms(9_000)), 14);
        assert_eq!(mood.points(ms(11_999)), 14);
        assert_eq!(mood.points(ms(12_000)), 13);
    }

    #[test]
    fn points_stay_in_range() {
        let mut mood = Mood::new(ms(0));
        mood.add(ms(0), MOOD_CAUGHT);
        assert_eq!(mood.points(ms(0)), 0);
        for _ in 0..10 {
            mood.add(ms(0), MOOD_SUCCESS);
        }
        assert_eq!(mood.points(ms(0)), MOOD_MAX);
    }
}
