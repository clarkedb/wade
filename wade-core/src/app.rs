//! `App`: the core's entry point. Owns all state and routes events to features.

use crate::character::{Expression, Wade};
use crate::event::{Event, EventKind, Key, Touch, TouchPhase};
use crate::input::TouchTracker;
use crate::layout::{self, Target, Tile};
use crate::rng::Rng;
use crate::settings::{Settings, SettingsButton};
use crate::time::{Duration, Instant};
use crate::timer::{Chime, Digits, TimerButton, TimerPhase, TimerState};
use crate::view::{BuddyView, LauncherView, SettingsView, View};

/// Frame interval while something is moving (about 30 fps).
pub const FRAME: Duration = Duration::from_millis(33);

/// A gap between events longer than this is beyond any platform's lateness.
/// Rather than replay every blink in it, Wade restarts his idle schedule.
pub const STALL_LIMIT: Duration = Duration::from_hours(1);

/// Maximum effects per `Output`; extras are dropped, never a panic (D17).
pub const MAX_EFFECTS: usize = 4;

/// Settings are saved once they have stopped changing for this long, so
/// stepping through several values writes storage once.
pub const SAVE_DELAY: Duration = Duration::from_secs(2);

/// For this long after the timer finishes, new touches are ignored: they were
/// aimed at what the screen showed before, and could otherwise dismiss a timer
/// its user never saw finish.
pub const TOUCH_GUARD: Duration = Duration::from_millis(500);

/// Side effects for the platform to carry out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Play the timer-finished chime, the tone sequence in
    /// [`crate::sound::CHIME`]. Emitted when the timer finishes and repeated
    /// while it stays Done, unless the chime setting is off (docs/ui.md#rules).
    Chime,
    /// Store these settings, for `App::new` at the next start. Emitted once
    /// they stop changing, or on leaving the Settings screen, and only when
    /// they differ from the last saved.
    SaveSettings(Settings),
}

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
    Launcher,
    Timer,
    Settings,
}

#[derive(Clone, Debug)]
pub struct App {
    now: Instant,
    screen: Screen,
    wade: Wade,
    timer: TimerState,
    /// Randomness for behavior, apart from Wade's motion (D21).
    rng: Rng,
    touch: TouchTracker,
    /// Touches that start before this are ignored (see `TOUCH_GUARD`).
    touch_guard_until: Instant,
    settings: Settings,
    /// The settings as last loaded or saved.
    saved: Settings,
    /// When to save the settings: `SAVE_DELAY` after the last change.
    save_at: Option<Instant>,
}

impl App {
    /// Start at `now`, seeding behavior with `seed`, with the `settings` the
    /// platform loaded.
    #[must_use]
    pub fn new(now: Instant, seed: u64, settings: Settings) -> App {
        App {
            now,
            screen: Screen::Buddy,
            wade: Wade::new(now, seed),
            timer: TimerState::Ready {
                set: settings.timer(),
            },
            rng: Rng::new(seed),
            touch: TouchTracker::new(),
            touch_guard_until: now,
            settings,
            saved: settings,
            save_at: None,
        }
    }

    #[must_use = "the platform must carry out the effects and honor redraw"]
    pub fn handle(&mut self, event: Event) -> Output {
        let mut out = Output::default();
        // Anything animating before or after this event needs a new frame.
        out.redraw |= self.animating();
        let digits = self.visible_digits();
        // Clamp: events may arrive slightly out of order.
        let target = self.now.max(event.at);
        self.advance_to(target, &mut out);
        out.redraw |= self.animating() || self.visible_digits() != digits;

        match event.kind {
            EventKind::Touch(touch) => self.on_touch(touch, &mut out),
            // Keys act only on the Buddy screen (docs/architecture.md#events).
            EventKind::Key(key) => {
                if self.screen == Screen::Buddy {
                    out.redraw |= self.on_key(key);
                }
            }
            EventKind::Deadline => {}
        }
        out
    }

    fn on_touch(&mut self, touch: Touch, out: &mut Output) {
        let pressed = self.pressed_button();
        let screen = self.screen;
        let row = self.timer.row();
        let tap = if touch.phase == TouchPhase::Down && self.now < self.touch_guard_until {
            self.touch.cancel();
            None
        } else {
            self.touch.handle(touch, |p| match screen {
                Screen::Buddy => layout::hit_buddy(p),
                Screen::Launcher => layout::hit_launcher(p),
                Screen::Timer => layout::hit_timer(p, row),
                Screen::Settings => layout::hit_settings(p),
            })
        };
        out.redraw |= self.pressed_button() != pressed;
        match tap {
            Some(Target::Wade) => out.redraw |= self.wade.on_tap(self.now),
            Some(Target::Apps) => self.switch_to(Screen::Launcher, out),
            Some(Target::Tile(Tile::Timer)) => self.switch_to(Screen::Timer, out),
            Some(Target::Tile(Tile::Settings)) => self.switch_to(Screen::Settings, out),
            Some(Target::Back) => self.back(out),
            Some(Target::Timer(TimerButton::Dismiss)) => self.dismiss(out),
            Some(Target::Timer(button)) => {
                if self.timer.press(button, self.now) {
                    out.redraw = true;
                    self.remember_duration();
                }
            }
            Some(Target::Settings(button)) => {
                self.change_settings(self.settings.toggled(button));
                out.redraw = true;
            }
            None => {}
        }
    }

    /// Back from a finished timer dismisses it, like Dismiss.
    fn back(&mut self, out: &mut Output) {
        match self.screen {
            Screen::Timer if self.timer.phase() == TimerPhase::Done => self.dismiss(out),
            Screen::Timer | Screen::Settings => self.switch_to(Screen::Launcher, out),
            Screen::Launcher | Screen::Buddy => self.switch_to(Screen::Buddy, out),
        }
    }

    /// Dismiss a finished timer and return to Buddy, so Wade can react.
    fn dismiss(&mut self, out: &mut Output) {
        if self.timer.press(TimerButton::Dismiss, self.now) {
            out.redraw |= self.wade.timer_dismissed(self.now, &mut self.rng);
        }
        self.switch_to(Screen::Buddy, out);
    }

    /// Apply `settings` at once. Unless that returns them to the last saved,
    /// they are saved once they stop changing.
    fn change_settings(&mut self, settings: Settings) {
        if settings == self.settings {
            return;
        }
        self.settings = settings;
        self.save_at = (settings != self.saved).then(|| self.now + SAVE_DELAY);
    }

    /// Store the timer's duration in the settings, so it survives a restart.
    fn remember_duration(&mut self) {
        if let Some(settings) = self.settings.with_timer(self.timer.duration()) {
            self.change_settings(settings);
        }
    }

    fn on_key(&mut self, key: Key) -> bool {
        match key {
            Key::Digit(digit) => self
                .wade
                .show(Expression::ALL[usize::from(digit.get())], self.now),
            Key::Z => self.wade.sleep(self.now),
            Key::P => {
                self.change_settings(self.settings.toggled(SettingsButton::EyeStyle));
                true
            }
        }
    }

    /// A chime fell due at `now`. The first brings up the Timer screen, from
    /// any screen, and wakes Wade if he is asleep, even with the chime off.
    fn on_chime(&mut self, chime: Chime, out: &mut Output) {
        if chime == Chime::First {
            self.wade.timer_finished(self.now);
            self.switch_to(Screen::Timer, out);
            self.touch_guard_until = self.now + TOUCH_GUARD;
        }
        // Chimes that fall due in one event, after a late wake, ring once.
        if self.settings.chime() && !out.effects.contains(&Effect::Chime) {
            let _ = out.effects.push(Effect::Chime);
        }
    }

    /// Show `screen`. A screen change cancels any touch in progress, so its
    /// `Up` cannot land on the new screen (docs/ui.md#touch-handling). The
    /// timer finishing does this even when the Timer screen is already shown.
    fn switch_to(&mut self, screen: Screen, out: &mut Output) {
        // Leaving the Settings screen saves at once.
        if self.screen == Screen::Settings && screen != Screen::Settings && self.save_at.is_some() {
            self.save(out);
        }
        self.screen = screen;
        self.touch.cancel();
        out.redraw = true;
    }

    /// Save the settings if they changed since they were last saved. With no
    /// room left for the effect, try again a frame later.
    fn save(&mut self, out: &mut Output) {
        self.save_at = None;
        if self.settings == self.saved {
            return;
        }
        if out
            .effects
            .push(Effect::SaveSettings(self.settings))
            .is_ok()
        {
            self.saved = self.settings;
        } else {
            self.save_at = Some(self.now + FRAME);
        }
    }

    /// The button under a touch in progress, drawn pressed. Wade has no pressed look.
    fn pressed_button(&self) -> Option<Target> {
        self.touch
            .pressed()
            .filter(|&target| target != Target::Wade)
    }

    /// The earliest scheduled transition across all features, including
    /// Wade's only if `include_wade`.
    fn next_transition(&self, include_wade: bool) -> Option<Instant> {
        let wade = include_wade
            .then(|| self.wade.next_transition(self.wade_visible()))
            .flatten();
        [self.timer.next_transition(), self.save_at, wade]
            .into_iter()
            .flatten()
            .min()
    }

    /// True while a visible feature's pose is changing with time.
    fn animating(&self) -> bool {
        self.wade_visible() && self.wade.animating(self.now)
    }

    fn wade_visible(&self) -> bool {
        self.screen == Screen::Buddy
    }

    /// The timer's digits, while the Timer screen shows them.
    fn visible_digits(&self) -> Option<Digits> {
        (self.screen == Screen::Timer).then(|| self.timer.digits(self.now))
    }

    /// Advance `now` to `target`, applying every timed transition that became due
    /// along the way in chronological order across features (docs/architecture.md#events).
    fn advance_to(&mut self, target: Instant, out: &mut Output) {
        // Past STALL_LIMIT, Wade restarts his idle schedule instead of replaying
        // it (D20). The timer's few transitions still replay first, in order,
        // and see Wade as of the last event. That matches an in-order replay
        // only while no timed transition changes whether he is asleep.
        let stalled = self
            .wade
            .next_transition(self.wade_visible())
            .is_some_and(|t| target.saturating_since(t) > STALL_LIMIT);
        let mut last = None;
        while let Some(t) = self.next_transition(!stalled) {
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
            // Features are applied in a fixed order to break ties at the same
            // instant: the timer first, because finishing switches screens,
            // which decides whether Wade is visible.
            if let Some(chime) = self.timer.advance(self.now) {
                self.on_chime(chime, out);
            }
            if !stalled {
                let visible = self.wade_visible();
                out.redraw |= self.wade.advance(self.now, visible);
            }
            if self.save_at.is_some_and(|t| t <= self.now) {
                self.save(out);
            }
        }
        if stalled {
            self.wade.fast_forward(target);
            out.redraw |= self.wade_visible();
        }
        self.now = target;
        self.wade.catch_up(target);
    }

    /// The earliest time the core needs a `Deadline` event, or `None` if
    /// nothing visible will change without input and no save is pending.
    /// Always later than the last handled event.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        // Hidden, Wade requests nothing: his schedule catches up at the next
        // event (docs/architecture.md#hidden-features).
        let wade = self
            .wade_visible()
            .then(|| self.wade.next_transition(true))
            .flatten();
        let tick = (self.screen == Screen::Timer)
            .then(|| self.timer.next_tick(self.now))
            .flatten();
        let frame = self.animating().then(|| self.now + FRAME);
        let deadline = [
            wade,
            self.timer.next_transition(),
            self.save_at,
            tick,
            frame,
        ]
        .into_iter()
        .flatten()
        .min()?;
        debug_assert!(
            deadline > self.now || self.now == Instant::MAX,
            "deadline {deadline:?} is not after now {:?}",
            self.now
        );
        // At Instant::MAX nothing can be scheduled later.
        (deadline > self.now).then_some(deadline)
    }

    /// Settings changed since they were last saved, for a platform about to
    /// stop to store.
    #[must_use]
    pub fn unsaved_settings(&self) -> Option<Settings> {
        (self.settings != self.saved).then_some(self.settings)
    }

    /// What is on screen, as plain data, as of the last handled event.
    #[must_use]
    pub fn view(&self) -> View {
        match self.screen {
            Screen::Buddy => View::Buddy(BuddyView {
                expression: self.wade.expression(),
                asleep: self.wade.asleep(),
                blinking: self.wade.blinking(self.now),
                pose: self.wade.pose(self.now),
                eye_style: self.settings.eye_style(),
                color: self.settings.color(),
                apps_pressed: self.touch.pressed() == Some(Target::Apps),
            }),
            Screen::Launcher => View::Launcher(LauncherView {
                pressed: self.pressed_button(),
            }),
            Screen::Timer => View::Timer(self.timer.view(self.now, self.pressed_button())),
            Screen::Settings => View::Settings(SettingsView {
                settings: self.settings,
                pressed: self.pressed_button(),
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

    #[must_use]
    pub const fn settings(&self) -> Settings {
        self.settings
    }

    /// The timer, which runs on every screen, for tests and the state hash.
    #[cfg(any(test, feature = "harness"))]
    #[must_use]
    pub const fn timer_state(&self) -> TimerState {
        self.timer
    }

    /// Wade's state, on every screen, for the state hash.
    #[cfg(any(test, feature = "harness"))]
    pub(crate) const fn wade(&self) -> &Wade {
        &self.wade
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
        let app = App::new(ms(500), SEED, Settings::DEFAULT);
        assert_eq!(app.now(), ms(500));
        assert_eq!(app.screen(), Screen::Buddy);
        assert_eq!(app.timer_state(), TimerState::new());
    }

    #[test]
    fn handle_moves_now_to_the_event_time() {
        let mut app = App::new(ms(0), SEED, Settings::DEFAULT);
        let _ = app.handle(Event::deadline(ms(1_000)));
        assert_eq!(app.now(), ms(1_000));
        let _ = app.handle(Event::touch(ms(1_500), TouchPhase::Down, Point::zero()));
        assert_eq!(app.now(), ms(1_500));
    }

    #[test]
    fn late_event_does_not_move_now_backwards() {
        let mut app = App::new(ms(0), SEED, Settings::DEFAULT);
        let _ = app.handle(Event::deadline(ms(1_000)));
        let _ = app.handle(Event::touch(ms(400), TouchPhase::Down, Point::zero()));
        assert_eq!(app.now(), ms(1_000));
    }

    #[test]
    fn idle_deadline_has_no_effects_and_no_redraw() {
        // Once the eyes have opened, nothing moves until the first glance, at least 1.2 s in.
        let mut app = App::new(ms(0), SEED, Settings::DEFAULT);
        let _ = app.handle(Event::deadline(ms(500)));
        assert_eq!(app.handle(Event::deadline(ms(600))), Output::default());
    }

    #[test]
    fn no_deadline_once_time_runs_out() {
        // Schedules saturate at Instant::MAX, where no deadline can be later than now.
        let mut app = App::new(ms(u64::MAX - 10_000), SEED, Settings::DEFAULT);
        let _ = app.handle(Event::deadline(Instant::MAX));
        assert_eq!(app.now(), Instant::MAX);
        assert_eq!(app.next_deadline(), None);
    }

    #[test]
    fn a_stall_still_finishes_the_timer_and_rings_once() {
        let mut app = App::new(ms(0), SEED, Settings::DEFAULT);
        app.timer = TimerState::Running {
            set: Duration::from_mins(1),
            ends_at: ms(60_000),
        };
        let late = ms(60_000) + STALL_LIMIT + STALL_LIMIT;
        let out = app.handle(Event::deadline(late));
        assert_eq!(out.effects.as_slice(), [Effect::Chime]);
        assert!(out.redraw);
        assert_eq!(app.screen(), Screen::Timer);
        assert!(matches!(
            app.timer_state(),
            TimerState::Done { since, chimes: 10, .. } if since == ms(60_000)
        ));
    }
}
