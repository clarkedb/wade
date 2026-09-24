//! `App`: the core's entry point. Owns all state and routes events to features.

use crate::character::Wade;
use crate::event::{Event, EventKind};
use crate::input::TouchTracker;
use crate::layout::{self, Target};
use crate::time::{Duration, Instant};
use crate::view::{BuddyView, View};

/// Frame interval while something is moving (about 30 fps).
pub const FRAME: Duration = Duration::from_millis(33);

/// A gap between events longer than this is beyond any platform's lateness.
/// Rather than replay every blink in it, Wade restarts his idle schedule.
pub const STALL_LIMIT: Duration = Duration::from_hours(1);

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
}

impl App {
    #[must_use]
    pub fn new(now: Instant, seed: u64) -> App {
        App {
            now,
            screen: Screen::Buddy,
            wade: Wade::new(now, seed),
            touch: TouchTracker::new(),
        }
    }

    #[must_use = "the platform must carry out the effects and honor redraw"]
    pub fn handle(&mut self, event: Event) -> Output {
        let mut out = Output::default();
        // Anything animating before or after this event needs a new frame.
        out.redraw |= self.animating();
        // Clamp: events may arrive slightly out of order.
        let target = self.now.max(event.at);
        self.advance_to(target, &mut out);
        out.redraw |= self.animating();

        match event.kind {
            EventKind::Touch(touch) => {
                let screen = self.screen;
                let tap = self.touch.handle(touch, |p| match screen {
                    Screen::Buddy => layout::hit_buddy(p),
                });
                if let Some(Target::Wade) = tap {
                    out.redraw |= self.wade.on_tap(self.now);
                }
            }
            EventKind::Deadline => {}
        }
        out
    }

    /// The earliest scheduled transition across all features.
    fn next_transition(&self) -> Option<Instant> {
        self.wade.next_transition(self.wade_visible())
    }

    /// True while a visible feature's pose is changing with time.
    fn animating(&self) -> bool {
        self.wade_visible() && self.wade.animating(self.now)
    }

    fn wade_visible(&self) -> bool {
        self.screen == Screen::Buddy
    }

    /// Advance `now` to `target`, applying every timed transition that became due
    /// along the way in chronological order across features (docs/architecture.md#events).
    fn advance_to(&mut self, target: Instant, out: &mut Output) {
        if self
            .next_transition()
            .is_some_and(|t| target.saturating_since(t) > STALL_LIMIT)
        {
            self.wade.fast_forward(target);
            out.redraw |= self.wade_visible();
        }
        let mut last = None;
        while let Some(t) = self.next_transition() {
            if t > target {
                break;
            }
            if last == Some(t) {
                // No feature moved past `t`. Legitimate only once schedules saturate.
                debug_assert!(t == Instant::MAX, "no feature advanced past {t:?}");
                break;
            }
            debug_assert!(
                t >= self.now,
                "stale transition {t:?} before now {:?}",
                self.now
            );
            last = Some(t);
            self.now = self.now.max(t);
            // Visibility is re-read each step: a transition can switch screens (M2).
            // Features are applied in a fixed order to break ties at the same instant.
            let visible = self.wade_visible();
            out.redraw |= self.wade.advance(self.now, visible);
        }
        self.now = target;
    }

    /// The earliest time the core needs a `Deadline` event, or `None` if nothing
    /// will change without input. Always later than the last handled event.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        let frame = self.animating().then(|| self.now + FRAME);
        let deadline = self.next_transition().into_iter().chain(frame).min()?;
        debug_assert!(
            deadline > self.now || self.now == Instant::MAX,
            "deadline {deadline:?} is not after now {:?}",
            self.now
        );
        // At Instant::MAX nothing can be scheduled later.
        (deadline > self.now).then_some(deadline)
    }

    /// What is on screen, as plain data, as of the last handled event.
    #[must_use]
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
    #[must_use]
    pub const fn now(&self) -> Instant {
        self.now
    }

    #[must_use]
    pub const fn screen(&self) -> Screen {
        self.screen
    }
}

#[cfg(test)]
mod tests {
    use embedded_graphics::geometry::Point;

    use super::*;
    use crate::event::TouchPhase;

    const SEED: u64 = 42;

    fn ms(ms: u64) -> Instant {
        Instant::from_millis(ms)
    }

    #[test]
    fn starts_on_buddy_at_the_given_time() {
        let app = App::new(ms(500), SEED);
        assert_eq!(app.now(), ms(500));
        assert_eq!(app.screen(), Screen::Buddy);
    }

    #[test]
    fn handle_moves_now_to_the_event_time() {
        let mut app = App::new(ms(0), SEED);
        let _ = app.handle(Event::deadline(ms(1_000)));
        assert_eq!(app.now(), ms(1_000));
        let _ = app.handle(Event::touch(ms(1_500), TouchPhase::Down, Point::zero()));
        assert_eq!(app.now(), ms(1_500));
    }

    #[test]
    fn late_event_does_not_move_now_backwards() {
        let mut app = App::new(ms(0), SEED);
        let _ = app.handle(Event::deadline(ms(1_000)));
        let _ = app.handle(Event::touch(ms(400), TouchPhase::Down, Point::zero()));
        assert_eq!(app.now(), ms(1_000));
    }

    #[test]
    fn idle_deadline_has_no_effects_and_no_redraw() {
        // Once the eyes have opened, nothing moves until the first glance, at least 1.2 s in.
        let mut app = App::new(ms(0), SEED);
        let _ = app.handle(Event::deadline(ms(500)));
        assert_eq!(app.handle(Event::deadline(ms(600))), Output::default());
    }

    #[test]
    fn no_deadline_once_time_runs_out() {
        // Schedules saturate at Instant::MAX, where no deadline can be later than now.
        let mut app = App::new(ms(u64::MAX - 10_000), SEED);
        let _ = app.handle(Event::deadline(Instant::MAX));
        assert_eq!(app.now(), Instant::MAX);
        assert_eq!(app.next_deadline(), None);
    }
}
