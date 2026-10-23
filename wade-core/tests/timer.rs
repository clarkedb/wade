//! Timer behavior (docs/roadmap.md#m2-timer, docs/ui.md#timer).

mod common;

use std::time::Duration;

use common::{SEED, back, button, home, ms, open};
use embedded_graphics::geometry::Point;
use wade_core::app::{COUNTDOWN, Screen, TOUCH_GUARD};
use wade_core::character::{
    Expression, GROGGY_DURATION, HAPPY_DURATION, TIMER_REACTION, WAKE_DURATION,
};
use wade_core::harness::Harness;
use wade_core::layout::{self, Target, Tile, WADE_CENTER};
use wade_core::settings::Volume;
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

/// Open the Timer screen at 1 s, set 1:00, start it at 2 s, and go back to
/// the Launcher at 3 s. It finishes at 62 s, away from the Timer screen and
/// from Wade.
fn one_minute_timer_from_the_launcher() -> Harness {
    let mut h = Harness::new(SEED);
    set_one_minute(&mut h, 1_000);
    h.tap(ms(2_000), button(TimerButton::Start));
    h.tap(ms(3_000), back());
    h
}

/// Like `one_minute_timer_from_the_launcher`, but handed to Wade at 3 s, so
/// he keeps time on Buddy.
fn one_minute_timer_with_wade() -> Harness {
    let mut h = Harness::new(SEED);
    set_one_minute(&mut h, 1_000);
    h.tap(ms(2_000), button(TimerButton::Start));
    h.tap(ms(3_000), button(TimerButton::Watch));
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
fn finishing_elsewhere_switches_to_the_timer_on_time() {
    let mut h = one_minute_timer_from_the_launcher();
    // run_until panics if the timer finishes without a requested deadline.
    h.run_until(before(ENDS_AT, 1));
    assert_eq!(h.app.screen(), Screen::Launcher);
    h.run_until(ENDS_AT);
    assert_eq!(h.app.screen(), Screen::Timer);
    assert_eq!(h.timer().phase, TimerPhase::Done);
    assert_eq!(chimes(&h), 1);
}

#[test]
fn chimes_repeat_on_the_interval_ten_times() {
    let mut h = one_minute_timer_from_the_launcher();
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
fn a_late_repeat_rings_once_and_keeps_the_original_schedule() {
    for volume in Volume::ALL {
        let audible = volume != Volume::Off;
        let mut h = Harness::with_settings(SEED, Settings::DEFAULT.with_volume(volume));
        set_one_minute(&mut h, 1_000);
        h.tap(ms(2_000), button(TimerButton::Start));
        h.tap(ms(3_000), back());
        h.run_until(ENDS_AT);
        assert_eq!(chimes(&h), usize::from(audible));

        // Miss two repeats, then wake between the second and third.
        let late = after(chime(2), 500);
        let out = h.handle(Event::deadline(late));
        assert_eq!(
            out.effects.iter().filter(|&&e| e == Effect::Chime).count(),
            usize::from(audible)
        );
        assert_eq!(chimes(&h), 2 * usize::from(audible));
        assert!(h.handle(Event::deadline(late)).effects.is_empty());

        if audible {
            assert_eq!(h.app.next_deadline(), Some(chime(3)));
            h.run_until(before(chime(3), 1));
            assert_eq!(chimes(&h), 2);
            h.run_until(chime(3));
            assert_eq!(chimes(&h), 3);
        }
    }
}

#[test]
fn dismissing_stops_the_chimes() {
    let mut h = one_minute_timer_from_the_launcher();
    h.run_until(chime(2));
    assert_eq!(chimes(&h), 3);
    h.tap(chime(2) + CHIME_INTERVAL / 2, button(TimerButton::Dismiss));
    h.run_until(after(ENDS_AT, 300_000));
    assert_eq!(chimes(&h), 3);
}

#[test]
fn a_touch_in_progress_when_the_timer_finishes_is_ignored() {
    // On the back button, which dismisses once the Timer screen shows Done.
    let mut h = one_minute_timer_from_the_launcher();
    h.touch(before(ENDS_AT, 1_000), TouchPhase::Down, back());
    h.run_until(ENDS_AT);
    assert_eq!(h.timer().pressed, None, "back still pressed");
    h.touch(after(ENDS_AT, 1_000), TouchPhase::Up, back());
    assert_eq!(h.app.screen(), Screen::Timer);
    assert_eq!(h.timer().phase, TimerPhase::Done);
}

#[test]
fn a_touch_that_starts_just_after_the_timer_finishes_is_ignored() {
    let mut h = one_minute_timer_from_the_launcher();
    // Aimed at the Launcher's back button, which dismisses by the time it lands.
    let point = back();
    h.touch(after(ENDS_AT, 150), TouchPhase::Down, point);
    assert_eq!(h.timer().pressed, None);
    h.touch(after(ENDS_AT, 200), TouchPhase::Up, point);
    assert_eq!(h.timer().phase, TimerPhase::Done);
    h.tap(ENDS_AT + TOUCH_GUARD, point);
    assert_eq!(h.app.screen(), Screen::Buddy);
}

#[test]
fn a_repeat_chime_does_not_cancel_a_press() {
    let mut h = one_minute_timer_from_the_launcher();
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
fn on_buddy_the_countdown_ticks_every_second() {
    let mut h = one_minute_timer_with_wade();
    let mut deadlines = Vec::new();
    while let Some(d) = h.app.next_deadline().filter(|&d| d < ENDS_AT) {
        h.handle(Event::deadline(d));
        let clock = h.buddy().clock.expect("the countdown shows");
        assert_eq!(clock.digits, h.app.timer_state().digits(d), "at {d:?}");
        deadlines.push(d);
    }
    for k in 1..=58 {
        let tick = before(ENDS_AT, k * 1_000);
        assert!(deadlines.contains(&tick), "no tick at {tick:?}");
    }
    assert_eq!(h.app.next_deadline(), Some(ENDS_AT));
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
    let mut h = one_minute_timer_from_the_launcher();
    dismissing_makes_wade(&mut h, button(TimerButton::Dismiss), Expression::Proud);
}

#[test]
fn back_while_done_dismisses() {
    let mut h = one_minute_timer_from_the_launcher();
    dismissing_makes_wade(&mut h, back(), Expression::Proud);
}

#[test]
fn the_power_button_dismisses_a_finished_timer() {
    let mut h = one_minute_timer_from_the_launcher();
    h.power_button(after(ENDS_AT, 5_000));
    assert_eq!(h.app.timer_state(), TimerState::Ready { set: MIN_SET });
    assert_eq!(h.app.screen(), Screen::Buddy);
    assert_eq!(h.buddy().expression, Expression::Proud);
    assert!(!h.buddy().asleep);
}

#[test]
fn the_power_button_just_after_the_timer_finishes_is_ignored() {
    let mut h = one_minute_timer_from_the_launcher();
    h.power_button(after(ENDS_AT, 200));
    assert_eq!(h.timer().phase, TimerPhase::Done);
    h.power_button(ENDS_AT + TOUCH_GUARD);
    assert_eq!(h.app.screen(), Screen::Buddy);
}

/// Put Wade to sleep, then run a 1:00 timer from the Launcher that wakes him at
/// `ENDS_AT`, and dismiss it 5 s later. Returns when it was dismissed.
fn dismiss_a_timer_that_woke_him(h: &mut Harness) -> Instant {
    h.key(ms(500), Key::Z);
    set_one_minute(h, 1_000);
    h.tap(ms(2_000), button(TimerButton::Start));
    h.tap(ms(3_000), back());
    assert!(h.wade_asleep());
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

#[test]
fn the_watch_button_hands_the_timer_to_wade() {
    let h = one_minute_timer_with_wade();
    assert_eq!(h.app.screen(), Screen::Buddy);
    let clock = h.buddy().clock.expect("the countdown shows");
    assert_eq!(clock.phase, TimerPhase::Running);
    assert_eq!(clock.digits.to_string(), "00:59");
}

#[test]
fn the_countdown_shows_on_buddy_only_while_the_timer_is_going() {
    let mut h = Harness::new(SEED);
    assert_eq!(h.buddy().clock, None);
    set_one_minute(&mut h, 1_000);
    home(&mut h, ms(2_000));
    assert_eq!(h.buddy().clock, None, "shown while Ready");
    // Tapping where the countdown would be does nothing.
    h.tap(ms(2_500), layout::CLOCK.center());
    assert_eq!(h.app.screen(), Screen::Buddy);
}

#[test]
fn tapping_the_countdown_opens_the_timer() {
    let mut h = one_minute_timer_with_wade();
    h.tap(ms(10_000), layout::CLOCK.center());
    assert_eq!(h.app.screen(), Screen::Timer);
    assert_eq!(h.timer().phase, TimerPhase::Running);
}

#[test]
fn wade_is_intent_on_the_last_seconds() {
    let mut h = one_minute_timer_with_wade();
    let countdown = before(ENDS_AT, 10_000);
    assert_eq!(COUNTDOWN, Duration::from_secs(10));
    h.run_until(before(countdown, 1));
    assert_eq!(h.buddy().expression, Expression::Neutral);
    h.run_until(countdown);
    assert_eq!(h.buddy().expression, Expression::Focused);
    // A tap distracts him only briefly.
    let tap = after(countdown, 1_000);
    h.tap(tap, WADE_CENTER);
    assert_eq!(h.buddy().expression, Expression::Happy);
    h.run_until(tap + HAPPY_DURATION);
    assert_eq!(h.buddy().expression, Expression::Focused);
}

#[test]
fn a_beat_in_the_last_seconds_ends_back_on_the_countdown() {
    let mut h = one_minute_timer_with_wade();
    let countdown = before(ENDS_AT, 10_000);
    h.run_until(after(countdown, 500));
    h.key(after(countdown, 1_000), Key::B);
    assert_eq!(h.buddy().expression, Expression::Thinking);
    h.run_until(after(countdown, 6_000));
    assert_eq!(h.buddy().expression, Expression::Focused);
}

#[test]
fn the_timer_finishing_cuts_a_beat_short() {
    let mut h = one_minute_timer_with_wade();
    h.run_until(before(ENDS_AT, 500));
    h.key(before(ENDS_AT, 500), Key::B);
    assert_eq!(h.buddy().expression, Expression::Thinking);
    h.run_until(ENDS_AT);
    assert_eq!(h.buddy().expression, Expression::Surprised);
    h.run_until(after(ENDS_AT, 10_000));
    assert_eq!(h.buddy().expression, Expression::Proud);
}

#[test]
fn pausing_ends_his_watch() {
    let mut h = one_minute_timer_with_wade();
    let countdown = before(ENDS_AT, 10_000);
    h.run_until(after(countdown, 500));
    assert_eq!(h.buddy().expression, Expression::Focused);
    h.tap(after(countdown, 1_000), layout::CLOCK.center());
    h.tap(after(countdown, 1_000), button(TimerButton::Pause));
    home(&mut h, after(countdown, 2_000));
    assert_eq!(h.buddy().expression, Expression::Neutral);
    assert_eq!(
        h.buddy().clock.map(|clock| clock.phase),
        Some(TimerPhase::Paused)
    );
}

#[test]
fn finishing_while_wade_keeps_time_stays_on_buddy() {
    let mut h = one_minute_timer_with_wade();
    h.run_until(ENDS_AT);
    assert_eq!(h.app.screen(), Screen::Buddy);
    assert_eq!(chimes(&h), 1);
    assert_eq!(h.buddy().expression, Expression::Surprised);
    let clock = h.buddy().clock.expect("the countdown shows");
    assert_eq!(clock.phase, TimerPhase::Done);
    assert_eq!(clock.digits.to_string(), "00:00");
    h.run_until(ENDS_AT + WAKE_DURATION);
    assert_eq!(h.buddy().expression, Expression::Proud);
    // Proud until dismissed, however long the chimes run.
    h.run_until(after(ENDS_AT, 60_000));
    assert_eq!(h.buddy().expression, Expression::Proud);
}

#[test]
fn a_tap_on_wade_or_the_countdown_dismisses_a_timer_he_kept() {
    for point in [WADE_CENTER, layout::CLOCK.center()] {
        let mut h = one_minute_timer_with_wade();
        dismissing_makes_wade(&mut h, point, Expression::Proud);
        assert_eq!(h.buddy().clock, None);
    }
}

#[test]
fn a_touch_on_wade_in_progress_when_he_finishes_is_ignored() {
    let mut h = one_minute_timer_with_wade();
    h.touch(before(ENDS_AT, 1_000), TouchPhase::Down, WADE_CENTER);
    h.run_until(ENDS_AT);
    h.touch(after(ENDS_AT, 1_000), TouchPhase::Up, WADE_CENTER);
    assert_eq!(h.app.timer_state().phase(), TimerPhase::Done);
}

#[test]
fn a_tap_on_wade_just_after_he_finishes_is_ignored() {
    let mut h = one_minute_timer_with_wade();
    h.tap(after(ENDS_AT, 100), WADE_CENTER);
    assert_eq!(h.app.timer_state().phase(), TimerPhase::Done);
    h.tap(ENDS_AT + TOUCH_GUARD, WADE_CENTER);
    assert_eq!(h.app.timer_state().phase(), TimerPhase::Ready);
}

#[test]
fn wade_sometimes_dozes_off_on_the_job_and_is_flustered_when_caught() {
    let (mut dozed, mut stayed) = (0, 0);
    for seed in 0..40 {
        let mut h = Harness::new(seed);
        open(&mut h, ms(1_000), Tile::Timer);
        h.tap(ms(2_000), button(TimerButton::Start));
        h.tap(ms(3_000), button(TimerButton::Watch));
        let ends_at = ms(302_000);
        let mut slept = false;
        while let Some(d) = h.app.next_deadline().filter(|&d| d < ends_at) {
            h.handle(Event::deadline(d));
            slept |= h.buddy().asleep;
        }
        h.run_until(ends_at + WAKE_DURATION);
        let expected = if slept {
            dozed += 1;
            Expression::Flustered
        } else {
            stayed += 1;
            Expression::Proud
        };
        assert_eq!(h.buddy().expression, expected, "seed {seed}");
    }
    assert!(
        dozed >= 5 && stayed >= 15,
        "{dozed} dozed off and {stayed} stayed awake"
    );
}

/// How many of 60 seeds doze off keeping a 5:00 timer, after `taps` on Wade.
fn dozes_after(taps: u64) -> usize {
    (0..60)
        .filter(|&seed| {
            let mut h = Harness::new(seed);
            for i in 0..taps {
                h.tap(ms(100 + i * 100), WADE_CENTER);
            }
            open(&mut h, ms(1_000), Tile::Timer);
            h.tap(ms(2_000), button(TimerButton::Start));
            h.tap(ms(3_000), button(TimerButton::Watch));
            let mut slept = false;
            while let Some(d) = h.app.next_deadline().filter(|&d| d < ms(302_000)) {
                h.handle(Event::deadline(d));
                slept |= h.buddy().asleep;
            }
            slept
        })
        .count()
}

#[test]
fn a_pleased_wade_dozes_off_more_often() {
    // Four taps leave him Pleased when he starts keeping time.
    let (plain, pleased) = (dozes_after(0), dozes_after(4));
    assert!(
        pleased > plain && pleased >= 24,
        "{plain} dozed off Plain and {pleased} Pleased, of 60"
    );
}

#[test]
fn wade_never_dozes_off_on_a_short_timer() {
    for seed in 0..40 {
        let mut h = Harness::new(seed);
        set_one_minute(&mut h, 1_000);
        h.tap(ms(2_000), button(TimerButton::Start));
        h.tap(ms(3_000), button(TimerButton::Watch));
        while let Some(d) = h.app.next_deadline().filter(|&d| d < ENDS_AT) {
            h.handle(Event::deadline(d));
            assert!(!h.buddy().asleep, "seed {seed} dozed at {d:?}");
        }
    }
}
