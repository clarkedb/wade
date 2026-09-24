//! Inputs to the core. Events report raw facts, never interpretations.

use embedded_graphics::geometry::Point;

use crate::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Event {
    pub at: Instant,
    pub kind: EventKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// A raw touch sample in logical screen coordinates (320×240, origin top-left).
    Touch(Touch),
    /// A key press from the desktop keyboard, for trying out Wade's looks.
    /// The device has no keys.
    Key(Key),
    /// A deadline requested through `App::next_deadline` has been reached.
    /// Carries no other meaning. May arrive late, or more often than requested.
    Deadline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// A number key shows the expression with that number
    /// (docs/character.md#expressions).
    Digit(Digit),
    /// Z puts Wade to sleep.
    Z,
}

/// A number key, 0–9.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Digit(u8);

impl Digit {
    /// The number key `n`, or `None` unless `n` is 0–9.
    #[must_use]
    pub const fn new(n: u8) -> Option<Digit> {
        if n <= 9 { Some(Digit(n)) } else { None }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Touch {
    pub phase: TouchPhase,
    pub point: Point,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TouchPhase {
    Down,
    Move,
    Up,
}

impl Event {
    #[must_use]
    pub const fn deadline(at: Instant) -> Self {
        Event {
            at,
            kind: EventKind::Deadline,
        }
    }

    #[must_use]
    pub const fn key(at: Instant, key: Key) -> Self {
        Event {
            at,
            kind: EventKind::Key(key),
        }
    }

    #[must_use]
    pub const fn touch(at: Instant, phase: TouchPhase, point: Point) -> Self {
        Event {
            at,
            kind: EventKind::Touch(Touch { phase, point }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_are_zero_to_nine() {
        assert_eq!(Digit::new(9).map(Digit::get), Some(9));
        assert_eq!(Digit::new(10), None);
    }

    #[test]
    fn constructors_build_the_matching_kind() {
        let at = Instant::from_millis(1_000);
        assert_eq!(Event::deadline(at).kind, EventKind::Deadline);

        let point = Point::new(160, 110);
        let touch = Event::touch(at, TouchPhase::Down, point);
        assert_eq!(touch.at, at);
        assert_eq!(
            touch.kind,
            EventKind::Touch(Touch {
                phase: TouchPhase::Down,
                point
            })
        );
    }
}
