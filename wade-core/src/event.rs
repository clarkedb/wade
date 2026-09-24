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
    /// A deadline requested through `App::next_deadline` has been reached.
    /// Carries no other meaning. May arrive late, or more often than requested.
    Deadline,
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
