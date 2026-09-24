//! Inputs to the core. Events report raw facts, never interpretations.

use embedded_graphics::geometry::Point;

use crate::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub at: Instant,
    pub kind: EventKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// A raw touch sample in logical screen coordinates (320×240, origin top-left).
    Touch(Touch),
    /// A deadline requested through `App::next_deadline` has been reached.
    /// Carries no other meaning. May arrive late, or more often than requested.
    Deadline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    pub const fn deadline(at: Instant) -> Self {
        Event {
            at,
            kind: EventKind::Deadline,
        }
    }

    pub const fn touch(at: Instant, phase: TouchPhase, point: Point) -> Self {
        Event {
            at,
            kind: EventKind::Touch(Touch { phase, point }),
        }
    }
}
