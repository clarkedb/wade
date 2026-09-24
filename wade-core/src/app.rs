//! `App`: the core's entry point. Owns all state and routes events to features.

use crate::character::Wade;
use crate::event::{Event, EventKind};
use crate::input::TouchTracker;
use crate::layout::{self, Target};
use crate::rng::Rng;
use crate::time::{Duration, Instant};
use crate::view::{BuddyView, View};

/// Frame interval while something is moving (about 30 fps).
pub const FRAME: Duration = Duration::from_millis(33);

/// Maximum effects per `Output`; extras are dropped, never a panic (D17).
pub const MAX_EFFECTS: usize = 4;

/// Side effects for the platform to carry out.
///
/// Empty in M1. `Chime` arrives in M2 (docs/architecture.md#planned-additions).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    /// Side effects for the platform to carry out, in order. If more than four
    /// are produced by one event, the extras are dropped (never a panic).
    pub effects: heapless::Vec<Effect, MAX_EFFECTS>,
    /// The view may have changed since the last render. A false positive costs
    /// one redundant frame; a false negative is a bug.
    pub redraw: bool,
}

/// Which screen is displayed and receives touch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Screen {
    #[default]
    Buddy,
}

#[derive(Clone, Debug)]
pub struct App {
    now: Instant,
    screen: Screen,
    wade: Wade,
    touch: TouchTracker,
    rng: Rng,
}

impl App {
    pub fn new(now: Instant, seed: u64) -> App {
        let mut rng = Rng::new(seed);
        let wade = Wade::new(now, &mut rng);
        App {
            now,
            screen: Screen::Buddy,
            wade,
            touch: TouchTracker::new(),
            rng,
        }
    }

    pub fn handle(&mut self, event: Event) -> Output {
        let mut out = Output::default();
        // Clamp: events may arrive slightly out of order.
        let target = self.now.max(event.at);
        self.advance_to(target, &mut out);

        if let EventKind::Touch(touch) = event.kind {
            let screen = self.screen;
            let tap = self.touch.handle(touch, |p| match screen {
                Screen::Buddy => layout::hit_buddy(p),
            });
            if let Some(Target::Wade) = tap {
                out.redraw |= self.wade.on_tap(self.now);
            }
        }
        out
    }

    /// Advance `now` to `target`, applying every timed transition that became due
    /// along the way in chronological order across features (docs/architecture.md#events).
    fn advance_to(&mut self, target: Instant, out: &mut Output) {
        let visible = self.screen == Screen::Buddy;
        while let Some(t) = self.wade.next_transition(visible) {
            if t > target {
                break;
            }
            self.now = t;
            // Features are applied in a fixed order to break ties at the same instant.
            out.redraw |= self.wade.advance(t, &mut self.rng, visible);
        }
        self.now = target;
    }

    /// The earliest time the core needs a `Deadline` event, or `None` if nothing
    /// will change without input. Always later than the last handled event.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.wade
            .next_transition(self.screen == Screen::Buddy)
            .map(|t| t.max(self.now + Duration::from_millis(1)))
    }

    /// What is on screen, as plain data, as of the last handled event.
    pub fn view(&self) -> View {
        match self.screen {
            Screen::Buddy => View::Buddy(BuddyView {
                expression: self.wade.expression(),
                blinking: self.wade.blinking(self.now),
                pose: self.wade.pose(self.now),
            }),
        }
    }

    /// The latest timestamp the core has seen.
    pub fn now(&self) -> Instant {
        self.now
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }
}
