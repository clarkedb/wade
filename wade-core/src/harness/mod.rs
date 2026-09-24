//! Test harness: drives `App` exactly as a platform does (docs/testing.md#harness).
//!
//! Behind the `harness` feature (which links std), so both the core's tests and
//! the desktop's replay mode can use it.

pub mod recording;

use std::vec::Vec;

use embedded_graphics::geometry::Point;

use crate::app::{App, Effect, Output, Screen};
use crate::character::Expression;
use crate::event::{Event, Key, TouchPhase};
use crate::time::Instant;
use crate::view::{BuddyView, View};

#[derive(Debug)]
pub struct Harness {
    pub app: App,
    /// Every effect emitted so far.
    pub effects: Vec<Effect>,
}

impl Harness {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            app: App::new(Instant::from_millis(0), seed),
            effects: Vec::new(),
        }
    }

    /// Deliver one event and record its effects.
    pub fn handle(&mut self, event: Event) -> Output {
        let out = self.app.handle(event);
        self.effects.extend(out.effects.iter().copied());
        out
    }

    /// Deliver a Deadline event at every deadline the app requests up to and
    /// including `t`, exactly as a platform would, then deliver one at `t`.
    ///
    /// # Panics
    ///
    /// If `t` is before the app's `now`, if a requested deadline is not later
    /// than `now`, or if the final Deadline at `t` changes discrete state. The
    /// last means the app changed at or before `t` without requesting a deadline
    /// for it, which a real platform would never have woken up for.
    pub fn run_until(&mut self, t: Instant) {
        assert!(
            t >= self.app.now(),
            "run_until({t:?}) is before now {:?}",
            self.app.now()
        );
        while let Some(deadline) = self.app.next_deadline() {
            if deadline > t {
                break;
            }
            assert!(
                deadline > self.app.now(),
                "next_deadline {deadline:?} is not later than now {:?}",
                self.app.now()
            );
            self.handle(Event::deadline(deadline));
        }
        let before = self.discrete();
        self.handle(Event::deadline(t));
        assert_eq!(
            before,
            self.discrete(),
            "state changed at {t:?} without a requested deadline"
        );
    }

    /// Run until `t`, then deliver a Down and an Up at `point`.
    pub fn tap(&mut self, t: Instant, point: Point) {
        self.run_until(t);
        self.handle(Event::touch(t, TouchPhase::Down, point));
        self.handle(Event::touch(t, TouchPhase::Up, point));
    }

    /// Run until `t`, then deliver a key press.
    pub fn key(&mut self, t: Instant, key: Key) {
        self.run_until(t);
        self.handle(Event::key(t, key));
    }

    /// Run until `t`, then deliver one raw touch sample.
    pub fn touch(&mut self, t: Instant, phase: TouchPhase, point: Point) {
        self.run_until(t);
        self.handle(Event::touch(t, phase, point));
    }

    /// The current view, which must be the Buddy screen.
    #[must_use]
    pub fn buddy(&self) -> BuddyView {
        match self.app.view() {
            View::Buddy(b) => b,
        }
    }

    /// Hash of the app's discrete state; see [`state_hash`].
    #[must_use]
    pub fn state_hash(&self) -> u32 {
        state_hash(&self.app)
    }

    fn discrete(&self) -> Discrete {
        let View::Buddy(buddy) = self.app.view();
        Discrete {
            screen: self.app.screen(),
            expression: buddy.expression,
            asleep: buddy.asleep,
            blinking: buddy.blinking,
        }
    }
}

/// Discrete state that must only change at a requested deadline or an input.
#[derive(Debug, PartialEq, Eq)]
struct Discrete {
    screen: Screen,
    expression: Expression,
    asleep: bool,
    blinking: bool,
}

/// A hash of discrete state only: the screen, the expression, and whether Wade
/// is asleep (later also the activity and the timer). Excludes `Pose` and
/// pixels, so tuning animation does not invalidate recordings (D16). FNV-1a, so
/// it is stable across platforms.
#[must_use]
pub fn state_hash(app: &App) -> u32 {
    let View::Buddy(buddy) = app.view();
    // Explicit codes, not `as u8`: reordering or inserting variants must not
    // change the hash of existing recordings. Never renumber these.
    let screen = match app.screen() {
        Screen::Buddy => 0u8,
    };
    let expression = match buddy.expression {
        Expression::Neutral => 0u8,
        Expression::Happy => 1,
        Expression::Sad => 2,
        Expression::Angry => 3,
        Expression::Surprised => 4,
        Expression::Sleepy => 5,
        Expression::Thinking => 6,
        Expression::Proud => 7,
        Expression::Focused => 8,
        Expression::Flustered => 9,
    };
    fnv1a(&[screen, expression, u8::from(buddy.asleep)])
}

fn fnv1a(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |h, &b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    })
}
