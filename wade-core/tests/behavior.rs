//! Behavior tests (docs/roadmap.md#m1-desktop-skeleton, docs/character.md#behavior-rules).

mod common;

use common::{OUTSIDE_WADE, SEED, ms};
use embedded_graphics::geometry::Point;
use wade_core::app::{FRAME, STALL_LIMIT};
use wade_core::character::{
    BLINK_DURATION, BLINK_INTERVAL_MAX, BLINK_INTERVAL_MIN, DOUBLE_BLINK_GAP, Expression,
};
use wade_core::harness::Harness;
use wade_core::layout::WADE_CENTER;
use wade_core::{Event, Instant, TouchPhase};

#[test]
fn starts_on_buddy_neutral() {
    let h = Harness::new(SEED);
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn tap_on_wade_makes_him_happy_for_two_seconds() {
    let mut h = Harness::new(SEED);
    h.tap(ms(1_000), WADE_CENTER);
    assert_eq!(h.buddy().expression, Expression::Happy);

    h.run_until(ms(2_999));
    assert_eq!(h.buddy().expression, Expression::Happy);

    h.run_until(ms(3_000));
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn second_tap_restarts_happy() {
    let mut h = Harness::new(SEED);
    h.tap(ms(1_000), WADE_CENTER);
    h.tap(ms(2_500), WADE_CENTER);

    h.run_until(ms(4_499));
    assert_eq!(h.buddy().expression, Expression::Happy);

    h.run_until(ms(4_500));
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn tap_outside_wade_does_nothing() {
    let mut h = Harness::new(SEED);
    h.tap(ms(1_000), OUTSIDE_WADE);
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn touch_that_moves_off_wade_is_not_a_tap() {
    let mut h = Harness::new(SEED);
    h.touch(ms(1_000), TouchPhase::Down, WADE_CENTER);
    h.touch(ms(1_050), TouchPhase::Move, OUTSIDE_WADE);
    h.touch(ms(1_100), TouchPhase::Up, OUTSIDE_WADE);
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn touch_that_moves_off_wade_and_back_is_not_a_tap() {
    let mut h = Harness::new(SEED);
    h.touch(ms(1_000), TouchPhase::Down, WADE_CENTER);
    h.touch(ms(1_050), TouchPhase::Move, OUTSIDE_WADE);
    h.touch(ms(1_100), TouchPhase::Move, WADE_CENTER);
    h.touch(ms(1_150), TouchPhase::Up, WADE_CENTER);
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn touch_that_moves_within_wade_is_a_tap() {
    let mut h = Harness::new(SEED);
    h.touch(ms(1_000), TouchPhase::Down, WADE_CENTER);
    h.touch(ms(1_050), TouchPhase::Move, WADE_CENTER + Point::new(10, 0));
    h.touch(ms(1_100), TouchPhase::Up, WADE_CENTER + Point::new(10, 0));
    assert_eq!(h.buddy().expression, Expression::Happy);
}

#[test]
fn first_blink_starts_between_two_and_six_seconds_and_lasts_120_ms() {
    let mut h = Harness::new(SEED);
    let start = (0..=6_000)
        .map(ms)
        .find(|&t| {
            h.run_until(t);
            h.buddy().blinking
        })
        .expect("no blink within 6 s");
    assert!(start >= ms(2_000), "first blink at {start:?}, before 2 s");

    let end = start + BLINK_DURATION;
    h.run_until(Instant::from_millis(end.as_millis() - 1));
    assert!(h.buddy().blinking, "blink ended before 120 ms");
    h.run_until(end);
    assert!(!h.buddy().blinking, "blink lasted longer than 120 ms");
}

/// Deliver every requested deadline up to `t`, calling `seen` after each.
fn follow_deadlines(h: &mut Harness, t: Instant, mut seen: impl FnMut(&Harness, Instant)) {
    while let Some(d) = h.app.next_deadline().filter(|&d| d <= t) {
        h.handle(Event::deadline(d));
        seen(h, d);
    }
}

#[test]
fn blinks_are_two_to_six_seconds_apart_or_one_double() {
    let mut h = Harness::new(SEED);
    let mut starts = Vec::new();
    let mut was_blinking = false;
    follow_deadlines(&mut h, ms(300_000), |h, t| {
        let blinking = h.buddy().blinking;
        if blinking && !was_blinking {
            starts.push(t);
        }
        was_blinking = blinking;
    });
    let double = BLINK_DURATION + DOUBLE_BLINK_GAP;
    let gaps: Vec<_> = starts
        .windows(2)
        .map(|w| w[1].saturating_since(w[0]))
        .collect();
    for &gap in &gaps {
        assert!(
            gap == double || (BLINK_INTERVAL_MIN..=BLINK_INTERVAL_MAX).contains(&gap),
            "blinks {gap:?} apart"
        );
    }
    assert!(gaps.contains(&double), "no double blink in 5 minutes");
    assert!(
        !gaps.windows(2).any(|w| w == [double, double]),
        "three blinks in a row"
    );
}

#[test]
fn a_stall_past_the_limit_applies_expiry_and_restarts_the_idle_schedule() {
    let mut h = Harness::new(SEED);
    h.tap(ms(1_000), WADE_CENTER);
    let late = ms(1_000) + STALL_LIMIT + STALL_LIMIT;
    h.handle(Event::deadline(late));
    assert_eq!(h.buddy().expression, Expression::Neutral);
    let next = h.app.next_deadline().expect("an idle schedule");
    assert!(
        next > late && next <= late + BLINK_INTERVAL_MAX,
        "next deadline {next:?} after a stall ending at {late:?}"
    );
}

#[test]
fn a_still_wade_wakes_well_below_the_frame_rate() {
    // Animating without end would wake the device on every frame.
    let frame_rate = 1_000 / FRAME.as_millis();
    let mut h = Harness::new(SEED);
    h.run_until(ms(5_000));
    let mut wakeups = 0;
    follow_deadlines(&mut h, ms(65_000), |_, _| wakeups += 1);
    let per_second = wakeups / 60;
    assert!(
        per_second <= 6,
        "{per_second} wakeups a second in Neutral (frames are {frame_rate})"
    );
}
