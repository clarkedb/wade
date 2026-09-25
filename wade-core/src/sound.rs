//! Tone sequences, which the platform synthesizes. There are no audio files.

use crate::time::Duration;

/// One note: a pitch held for a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tone {
    /// Pitch in hertz.
    pub hz: u16,
    pub duration: Duration,
}

impl Tone {
    #[must_use]
    pub const fn new(hz: u16, ms: u64) -> Self {
        Tone {
            hz,
            duration: Duration::from_millis(ms),
        }
    }
}

/// The timer-finished chime: a quick rising C major arpeggio that lands on a
/// held top note, bright and a little pleased with itself.
pub const CHIME: [Tone; 4] = [
    Tone::new(784, 110),  // G5
    Tone::new(1047, 110), // C6
    Tone::new(1319, 110), // E6
    Tone::new(1568, 420), // G6
];
