//! Test harness: drives `App` exactly as a platform does (docs/testing.md#harness).
//!
//! Behind the `harness` feature (which enables std), so both the core's tests and
//! the desktop's replay mode can use it.

pub mod recording;

use embedded_graphics::geometry::Point;

use crate::app::{App, Effect, Output, Screen};
use crate::event::{Event, TouchPhase};
use crate::time::Instant;
use crate::view::{BuddyView, View};

pub struct Harness {
    pub app: App,
    /// Every effect emitted so far.
    pub effects: Vec<Effect>,
}

impl Harness {
    pub fn new(seed: u64) -> Self {
        Harness {
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
    pub fn run_until(&mut self, t: Instant) {
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
        self.handle(Event::deadline(t));
    }

    /// Run until `t`, then deliver a Down and an Up at `point`.
    pub fn tap(&mut self, t: Instant, point: Point) {
        self.run_until(t);
        self.handle(Event::touch(t, TouchPhase::Down, point));
        self.handle(Event::touch(t, TouchPhase::Up, point));
    }

    /// Run until `t`, then deliver one raw touch sample.
    pub fn touch(&mut self, t: Instant, phase: TouchPhase, point: Point) {
        self.run_until(t);
        self.handle(Event::touch(t, phase, point));
    }

    /// The current view, which must be the Buddy screen.
    pub fn buddy(&self) -> BuddyView {
        match self.app.view() {
            View::Buddy(b) => b,
        }
    }

    /// Hash of the app's discrete state; see [`state_hash`].
    pub fn state_hash(&self) -> u32 {
        state_hash(&self.app)
    }
}

/// A hash of discrete state only: the screen and the expression (later also the
/// activity and the timer). Excludes `Pose` and pixels, so tuning animation does
/// not invalidate recordings (D16). FNV-1a, so it is stable across platforms.
pub fn state_hash(app: &App) -> u32 {
    let View::Buddy(buddy) = app.view();
    let screen = match app.screen() {
        Screen::Buddy => 0u8,
    };
    let expression = buddy.expression as u8;
    fnv1a(&[screen, expression])
}

fn fnv1a(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |h, &b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    })
}
