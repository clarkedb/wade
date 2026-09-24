//! M1 behavior tests (docs/roadmap.md#m1-desktop-skeleton).
//!
//! Remove each `#[ignore]` as the behavior lands.

mod common;

use common::{OUTSIDE_WADE, SEED, ms};
use embedded_graphics::geometry::Point;
use wade_core::TouchPhase;
use wade_core::character::{BLINK_DURATION, Expression};
use wade_core::harness::Harness;
use wade_core::layout::WADE_CENTER;

#[test]
fn starts_on_buddy_neutral() {
    let h = Harness::new(SEED);
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
#[ignore = "M1: not yet implemented"]
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
#[ignore = "M1: not yet implemented"]
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
#[ignore = "M1: not yet implemented"]
fn touch_that_moves_within_wade_is_a_tap() {
    let mut h = Harness::new(SEED);
    h.touch(ms(1_000), TouchPhase::Down, WADE_CENTER);
    h.touch(ms(1_050), TouchPhase::Move, WADE_CENTER + Point::new(10, 0));
    h.touch(ms(1_100), TouchPhase::Up, WADE_CENTER + Point::new(10, 0));
    assert_eq!(h.buddy().expression, Expression::Happy);
}

#[test]
#[ignore = "M1: not yet implemented"]
fn blinks_within_six_seconds_for_120_ms() {
    let mut h = Harness::new(SEED);
    let start = (0..=6_000)
        .find(|&t| {
            h.run_until(ms(t));
            h.buddy().blinking
        })
        .expect("no blink within 6 s");

    let end = start + BLINK_DURATION.as_millis() as u64;
    h.run_until(ms(end - 1));
    assert!(h.buddy().blinking, "blink ended before 120 ms");
    h.run_until(ms(end));
    assert!(!h.buddy().blinking, "blink lasted longer than 120 ms");
}
