//! Timer behavior (docs/roadmap.md#m2-timer, docs/ui.md#timer).

mod common;

use std::time::Duration;

use common::{SEED, apps, back, button, home, ms, open};
use embedded_graphics::geometry::Point;
use wade_core::app::{Screen, TOUCH_GUARD};
use wade_core::character::{Expression, GROGGY_DURATION, TIMER_REACTION};
use wade_core::harness::Harness;
use wade_core::layout::{Target, Tile, WADE_CENTER, WADE_FACE};
use wade_core::timer::{
    CHIME_INTERVAL, DEFAULT_SET, Digits, MAX_SET, MIN_SET, TimerButton, TimerPhase, TimerState,
};
use wade_core::{Effect, Event, Instant, Key, Settings, TouchPhase};

/// Open the Timer screen at `t` ms and shorten the timer to 1:00, finishing by `t + 500`.
fn set_one_minute(h: &mut Harness, t: u64) {
    open(h, ms(t), Tile::Timer);
    for step in 1..=4 {
        h.tap(ms(t + step * 100), button(TimerButton::Minus));
    }
    assert_eq!(h.app.timer_state().duration(), MIN_SET);
}

/// Open the Timer screen at 1 s, set 1:00, start it at 2 s, and return to
/// Buddy at 3 s. It finishes at 62 s.
fn one_minute_timer_from_buddy() -> Harness {
    let mut h = Harness::new(SEED);
    set_one_minute(&mut h, 1_000);
    h.tap(ms(2_000), button(TimerButton::Start));
    home(&mut h, ms(3_000));
    h
}

const ENDS_AT: Instant = Instant::from_millis(62_000);

/// When chime `k` falls, counting the first, at `ENDS_AT`, as zero.
fn chime(k: u32) -> Instant {
    ENDS_AT + CHIME_INTERVAL * k
}

fn chimes(h: &Harness) -> usize {
    h.effects.iter().filter(|&&e| e == Effect::Chime).count()
}

fn after(t: Instant, by: u64) -> Instant {
    t + Duration::from_millis(by)
}

fn before(t: Instant, by: u64) -> Instant {
    Instant::from_millis(t.as_millis() - by)
}

#[test]
fn start_runs_until_now_plus_set() {
    let mut h = Harness::new(SEED);
    open(&mut h, ms(1_000), Tile::Timer);
    assert_eq!(h.timer().phase, TimerPhase::Ready);
    assert_eq!(h.timer().digits.to_string(), "05:00");
    h.tap(ms(2_000), button(TimerButton::Start));
    assert_eq!(
        h.app.timer_state(),
        TimerState::Running {
            set: DEFAULT_SET,
            ends_at: ms(302_000),
        }
    );
}

#[test]
fn digits_round_up_to_whole_seconds() {
    let mut h = Harness::new(SEED);
    open(&mut h, ms(1_000), Tile::Timer);
    h.tap(ms(2_000), button(TimerButton::Start));
    h.run_until(ms(2_999));
    assert_eq!(h.timer().digits, Digits::new(5, 0), "299,001 ms left");
    h.run_until(ms(301_500));
    assert_eq!(h.timer().digits, Digits::new(0, 1), "500 ms left");
}

#[test]
fn pause_then_resume_keeps_the_time_left() {
    let mut h = Harness::new(SEED);
    open(&mut h, ms(1_000), Tile::Timer);
    h.tap(ms(2_000), button(TimerButton::Start));
    h.tap(ms(62_500), button(TimerButton::Pause));
    assert_eq!(h.timer().phase, TimerPhase::Paused);
    assert_eq!(h.timer().digits.to_string(), "04:00");
    h.run_until(ms(200_000));
    assert_eq!(h.timer().digits.to_string(), "04:00");
    h.tap(ms(200_000), button(TimerButton::Resume));
    assert_eq!(
        h.app.timer_state(),
        TimerState::Running {
            set: DEFAULT_SET,
            ends_at: ms(439_500),
        }
    );
}

#[test]
fn reset_returns_to_ready_with_the_same_duration() {
    let mut h = Harness::new(SEED);
    set_one_minute(&mut h, 1_000);
    h.tap(ms(2_000), button(TimerButton::Start));
    h.tap(ms(10_000), button(TimerButton::Pause));
    h.tap(ms(11_000), button(TimerButton::Reset));
    assert_eq!(h.app.timer_state(), TimerState::Ready { set: MIN_SET });
    assert_eq!(h.timer().digits.to_string(), "01:00");
}

#[test]
fn finishing_chimes_and_shows_done_on_the_timer_screen() {
    let mut h = Harness::new(SEED);
    set_one_minute(&mut h, 1_000);
    h.tap(ms(2_000), button(TimerButton::Start));
    h.run_until(before(ENDS_AT, 1));
    assert_eq!(chimes(&h), 0);
    assert_eq!(h.timer().phase, TimerPhase::Running);
    h.run_until(ENDS_AT);
    assert_eq!(chimes(&h), 1);
    assert_eq!(h.timer().phase, TimerPhase::Done);
}

#[test]
fn finishing_on_buddy_switches_to_the_timer_on_time() {
    let mut h = one_minute_timer_from_buddy();
    // run_until panics if the timer finishes without a requested deadline.
    h.run_until(before(ENDS_AT, 1));
    assert_eq!(h.app.screen(), Screen::Buddy);
    h.run_until(ENDS_AT);
    assert_eq!(h.app.screen(), Screen::Timer);
    assert_eq!(h.timer().phase, TimerPhase::Done);
    assert_eq!(chimes(&h), 1);
}

#[test]
fn chimes_repeat_on_the_interval_ten_times() {
    let mut h = one_minute_timer_from_buddy();
    for (k, rung) in (0..10).zip(0..) {
        h.run_until(before(chime(k), 1));
        assert_eq!(chimes(&h), rung, "early chime {k}");
        h.run_until(chime(k));
        assert_eq!(chimes(&h), rung + 1, "chime {k}");
    }
    h.run_until(after(ENDS_AT, 300_000));
    assert_eq!(chimes(&h), 10);
    assert_eq!(h.app.next_deadline(), None);
}

#[test]
fn dismissing_stops_the_chimes() {
    let mut h = one_minute_timer_from_buddy();
    h.run_until(chime(2));
    assert_eq!(chimes(&h), 3);
    h.tap(chime(2) + CHIME_INTERVAL / 2, button(TimerButton::Dismiss));
    h.run_until(after(ENDS_AT, 300_000));
    assert_eq!(chimes(&h), 3);
}

#[test]
fn a_touch_in_progress_when_the_timer_finishes_is_ignored() {
    // On Wade, over Dismiss once the Timer screen shows; and on the apps button.
    let over_dismiss = button(TimerButton::Dismiss);
    assert!(WADE_FACE.contains(over_dismiss));
    for point in [over_dismiss, apps()] {
        let mut h = one_minute_timer_from_buddy();
        h.touch(before(ENDS_AT, 1_000), TouchPhase::Down, point);
        h.run_until(ENDS_AT);
        assert_eq!(h.timer().pressed, None, "{point:?} still pressed");
        h.touch(after(ENDS_AT, 1_000), TouchPhase::Up, point);
        assert_eq!(h.app.screen(), Screen::Timer);
        assert_eq!(h.timer().phase, TimerPhase::Done);
    }
}

#[test]
fn a_touch_that_starts_just_after_the_timer_finishes_is_ignored() {
    let mut h = one_minute_timer_from_buddy();
    // Aimed at Wade, but over Dismiss by the time it lands.
    let point = button(TimerButton::Dismiss);
    h.touch(after(ENDS_AT, 150), TouchPhase::Down, point);
    assert_eq!(h.timer().pressed, None);
    h.touch(after(ENDS_AT, 200), TouchPhase::Up, point);
    assert_eq!(h.timer().phase, TimerPhase::Done);
    h.tap(ENDS_AT + TOUCH_GUARD, point);
    assert_eq!(h.app.screen(), Screen::Buddy);
}

#[test]
fn a_repeat_chime_does_not_cancel_a_press() {
    let mut h = one_minute_timer_from_buddy();
    let point = button(TimerButton::Dismiss);
    h.touch(before(chime(1), 50), TouchPhase::Down, point);
    h.touch(after(chime(1), 50), TouchPhase::Up, point);
    assert_eq!(chimes(&h), 2);
    assert_eq!(h.app.screen(), Screen::Buddy);
}

#[test]
fn holding_back_as_the_timer_finishes_does_not_dismiss_it() {
    let mut h = Harness::new(SEED);
    set_one_minute(&mut h, 1_000);
    h.tap(ms(2_000), button(TimerButton::Start));
    h.touch(before(ENDS_AT, 1_000), TouchPhase::Down, back());
    assert_eq!(h.timer().pressed, Some(Target::Back));
    h.run_until(ENDS_AT);
    assert_eq!(h.timer().pressed, None);
    h.touch(after(ENDS_AT, 1_000), TouchPhase::Up, back());
    assert_eq!(h.app.screen(), Screen::Timer);
    assert_eq!(h.timer().phase, TimerPhase::Done);
}

#[test]
fn hidden_wade_adds_no_deadlines_or_redraws_to_a_running_timer() {
    // Starting at 1:00, so no save of a new duration is pending.
    let one_minute = Settings::DEFAULT.with_timer(MIN_SET).expect("1 minute");
    let mut h = Harness::with_settings(SEED, one_minute);
    open(&mut h, ms(1_000), Tile::Timer);
    h.tap(ms(2_000), button(TimerButton::Start));
    let mut ticks = 0;
    while let Some(d) = h.app.next_deadline().filter(|&d| d < ENDS_AT) {
        assert_eq!(
            (ENDS_AT.as_millis() - d.as_millis()) % 1_000,
            0,
            "deadline {d:?} is not a tick"
        );
        let digits = h.timer().digits;
        let out = h.handle(Event::deadline(d));
        assert!(out.redraw && h.timer().digits != digits, "no tick at {d:?}");
        ticks += 1;
    }
    assert_eq!(ticks, 59);
    assert_eq!(h.app.next_deadline(), Some(ENDS_AT));
}

#[test]
fn deadlines_on_the_timer_screen_are_the_next_tick_or_the_end() {
    let mut h = Harness::new(SEED);
    set_one_minute(&mut h, 1_000);
    h.tap(ms(2_000), button(TimerButton::Start));
    assert_eq!(h.app.next_deadline(), Some(ms(3_000)));
    h.run_until(ms(2_500));
    assert_eq!(h.app.next_deadline(), Some(ms(3_000)));
    h.run_until(ms(61_000));
    assert_eq!(h.app.next_deadline(), Some(ENDS_AT));
}

#[test]
fn on_buddy_the_timer_adds_only_its_end_to_the_deadlines() {
    // Twins whose only difference is that one starts the timer.
    let mut running = one_minute_timer_from_buddy();
    let mut ready = Harness::new(SEED);
    set_one_minute(&mut ready, 1_000);
    home(&mut ready, ms(3_000));
    let follow = |h: &mut Harness| {
        let mut deadlines = Vec::new();
        while let Some(d) = h.app.next_deadline().filter(|&d| d < ENDS_AT) {
            h.handle(Event::deadline(d));
            deadlines.push(d);
        }
        deadlines
    };
    assert_eq!(follow(&mut running), follow(&mut ready));
    assert_eq!(running.app.next_deadline(), Some(ENDS_AT));
}

#[test]
fn leaving_and_returning_does_not_change_the_timer() {
    let mut h = Harness::new(SEED);
    open(&mut h, ms(1_000), Tile::Timer);
    h.tap(ms(2_000), button(TimerButton::Start));
    let timer = h.app.timer_state();
    home(&mut h, ms(10_000));
    open(&mut h, ms(20_000), Tile::Timer);
    assert_eq!(h.app.timer_state(), timer);
    assert_eq!(h.timer().digits.to_string(), "04:42");
}

#[test]
fn minus_and_plus_do_nothing_at_the_bounds() {
    let mut h = Harness::new(SEED);
    set_one_minute(&mut h, 1_000);
    assert_eq!(h.timer().row[0].map(|b| b.enabled), Some(false));
    h.touch(ms(2_000), TouchPhase::Down, button(TimerButton::Minus));
    assert_eq!(h.timer().pressed, None, "a dimmed button shows pressed");
    h.touch(ms(2_050), TouchPhase::Up, button(TimerButton::Minus));
    assert_eq!(h.app.timer_state().duration(), MIN_SET);

    for step in 0..98 {
        h.tap(ms(3_000 + step * 100), button(TimerButton::Plus));
    }
    assert_eq!(h.app.timer_state().duration(), MAX_SET);
    assert_eq!(h.timer().digits.to_string(), "99:00");
    assert_eq!(h.timer().row[2].map(|b| b.enabled), Some(false));
}

#[test]
fn a_pressed_row_button_shows_pressed() {
    let mut h = Harness::new(SEED);
    open(&mut h, ms(1_000), Tile::Timer);
    h.touch(ms(2_000), TouchPhase::Down, button(TimerButton::Start));
    assert_eq!(h.timer().pressed, Some(Target::Timer(TimerButton::Start)));
    h.touch(ms(2_050), TouchPhase::Up, button(TimerButton::Start));
    assert_eq!(h.timer().pressed, None);
    assert_eq!(h.timer().phase, TimerPhase::Running);
}

/// Tap `point` 5 s after the timer finishes at `ENDS_AT`, and check that it is
/// dismissed and Wade shows `expression` for `TIMER_REACTION`, then Neutral.
fn dismissing_makes_wade(h: &mut Harness, point: Point, expression: Expression) {
    let at = after(ENDS_AT, 5_000);
    h.tap(at, point);
    assert_eq!(h.app.timer_state(), TimerState::Ready { set: MIN_SET });
    assert_eq!(h.app.screen(), Screen::Buddy);
    assert_eq!(h.buddy().expression, expression);
    h.run_until(before(at + TIMER_REACTION, 1));
    assert_eq!(h.buddy().expression, expression);
    h.run_until(at + TIMER_REACTION);
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn dismiss_returns_to_buddy_and_wade_is_proud() {
    let mut h = one_minute_timer_from_buddy();
    dismissing_makes_wade(&mut h, button(TimerButton::Dismiss), Expression::Proud);
}

#[test]
fn back_while_done_dismisses() {
    let mut h = one_minute_timer_from_buddy();
    dismissing_makes_wade(&mut h, back(), Expression::Proud);
}

/// Put Wade to sleep, then run a 1:00 timer from Buddy that wakes him at
/// `ENDS_AT`, and dismiss it 5 s later. Returns when it was dismissed.
fn dismiss_a_timer_that_woke_him(h: &mut Harness) -> Instant {
    h.key(ms(500), Key::Z);
    set_one_minute(h, 1_000);
    h.tap(ms(2_000), button(TimerButton::Start));
    home(h, ms(3_000));
    assert!(h.buddy().asleep);
    h.run_until(ENDS_AT);
    assert!(!h.wade_asleep(), "the chime did not wake him");
    let at = after(ENDS_AT, 5_000);
    h.tap(at, back());
    at
}

#[test]
fn a_chime_that_wakes_wade_leaves_him_groggy_then_a_coin_decides() {
    let (mut woke, mut dozed) = (0, 0);
    for seed in 0..40 {
        let mut h = Harness::new(seed);
        let at = dismiss_a_timer_that_woke_him(&mut h);
        assert_eq!(h.app.screen(), Screen::Buddy);
        assert_eq!(h.buddy().expression, Expression::Sleepy);
        assert!(!h.buddy().asleep);
        h.run_until(before(at + GROGGY_DURATION, 1));
        assert_eq!(h.buddy().expression, Expression::Sleepy);
        assert!(!h.buddy().asleep);
        h.run_until(at + GROGGY_DURATION);
        if h.buddy().asleep {
            dozed += 1;
        } else {
            assert_eq!(h.buddy().expression, Expression::Neutral);
            woke += 1;
        }
    }
    assert!(
        woke >= 10 && dozed >= 10,
        "{woke} woke up and {dozed} dozed off"
    );
}

#[test]
fn only_the_timer_that_woke_him_leaves_him_groggy() {
    let mut h = Harness::new(SEED);
    let _ = dismiss_a_timer_that_woke_him(&mut h);
    // Tap him awake in case the coin put him back to sleep.
    h.tap(ms(80_000), WADE_CENTER);
    set_one_minute(&mut h, 82_000);
    h.tap(ms(83_000), button(TimerButton::Start));
    h.tap(ms(144_000), button(TimerButton::Dismiss));
    assert_eq!(h.buddy().expression, Expression::Proud);
}
