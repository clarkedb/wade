//! Screens and navigation (docs/ui.md#screens-and-navigation).

mod common;

use common::{SEED, apps, back, digit, ms};
use embedded_graphics::geometry::Point;
use wade_core::app::{FRAME, STALL_LIMIT, Screen};
use wade_core::character::Expression;
use wade_core::harness::Harness;
use wade_core::layout::{Target, WADE_CENTER};
use wade_core::{Event, Key, TouchPhase};

#[test]
fn apps_opens_the_timer_and_back_returns_to_buddy() {
    let mut h = Harness::new(SEED);
    h.tap(ms(1_000), apps());
    assert_eq!(h.app.screen(), Screen::Timer);
    h.tap(ms(2_000), back());
    assert_eq!(h.app.screen(), Screen::Buddy);
}

#[test]
fn a_touch_that_leaves_the_apps_button_does_not_open_the_timer() {
    let mut h = Harness::new(SEED);
    h.touch(ms(1_000), TouchPhase::Down, apps());
    assert!(h.buddy().apps_pressed);
    h.touch(ms(1_050), TouchPhase::Move, apps() - Point::new(40, 0));
    assert!(!h.buddy().apps_pressed);
    h.touch(ms(1_100), TouchPhase::Up, apps());
    assert_eq!(h.app.screen(), Screen::Buddy);
}

#[test]
fn back_shows_pressed_while_the_touch_stays_on_it() {
    let mut h = Harness::new(SEED);
    h.tap(ms(1_000), apps());
    assert!(
        h.handle(Event::touch(ms(2_000), TouchPhase::Down, back()))
            .redraw
    );
    assert_eq!(h.timer().pressed, Some(Target::Back));
    assert!(
        h.handle(Event::touch(ms(2_050), TouchPhase::Move, WADE_CENTER))
            .redraw
    );
    assert_eq!(h.timer().pressed, None);
}

#[test]
fn wade_ignores_taps_and_keys_on_the_timer_screen() {
    let mut h = Harness::new(SEED);
    h.tap(ms(1_000), apps());
    h.tap(ms(2_000), WADE_CENTER);
    h.key(ms(3_000), digit(3));
    h.key(ms(4_000), Key::Z);
    h.tap(ms(5_000), back());
    assert_eq!(h.buddy().expression, Expression::Neutral);
    assert!(!h.buddy().asleep);
}

#[test]
fn hidden_wade_requests_no_deadlines_and_never_redraws() {
    let mut h = Harness::new(SEED);
    // Happy expires, and blinks and glances fall due, while he is hidden.
    h.tap(ms(1_000), WADE_CENTER);
    h.tap(ms(1_500), apps());
    assert_eq!(h.app.next_deadline(), None);
    for t in (1_550..=30_000).step_by(50) {
        let out = h.handle(Event::deadline(ms(t)));
        assert!(!out.redraw, "redraw at {t} ms");
        assert_eq!(h.app.next_deadline(), None, "deadline requested at {t} ms");
    }
    h.tap(ms(30_000), back());
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn wade_returns_without_a_burst_of_catch_up_motion() {
    let mut h = Harness::new(SEED);
    h.tap(ms(1_000), apps());
    h.tap(ms(21_000), back());
    assert!(!h.buddy().blinking);
    assert_ne!(
        h.app.next_deadline(),
        Some(ms(21_000) + FRAME),
        "moving on return"
    );
}

#[test]
fn a_sleeping_wade_resumes_breathing_on_return() {
    let mut h = Harness::new(SEED);
    h.key(ms(1_000), Key::Z);
    h.tap(ms(3_000), apps());
    h.tap(ms(20_000), back());
    let next = h.app.next_deadline().expect("breathing steps");
    assert!(
        next > ms(20_000) && next <= ms(20_150),
        "next step at {next:?}"
    );
    h.run_until(ms(30_000));
    assert!(h.buddy().asleep);
}

#[test]
fn a_wake_inside_a_long_hidden_gap_changes_no_discrete_state() {
    // Hidden, Wade requests no deadlines, so the Timer screen can sit for over
    // STALL_LIMIT with no event. An extra wake inside that gap decides whether
    // he replays his idle schedule or restarts it (D25). Only motion may differ.
    let session = |extra_wake: bool| {
        let mut h = Harness::new(SEED);
        h.tap(ms(1_000), WADE_CENTER);
        h.tap(ms(1_500), apps());
        if extra_wake {
            h.handle(Event::deadline(ms(1_500) + STALL_LIMIT));
        }
        let back_at = ms(1_500) + STALL_LIMIT + STALL_LIMIT;
        h.tap(back_at, back());
        let mut hashes = vec![h.state_hash()];
        for s in 1..=30 {
            h.run_until(back_at + std::time::Duration::from_secs(s));
            hashes.push(h.state_hash());
        }
        (hashes, h.app.view())
    };
    let (replayed, replayed_view) = session(true);
    let (restarted, restarted_view) = session(false);
    assert_eq!(replayed, restarted);
    assert_ne!(
        replayed_view, restarted_view,
        "the gap no longer restarts his motion"
    );
}
