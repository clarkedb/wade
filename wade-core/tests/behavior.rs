//! Behavior tests (docs/roadmap.md#m1-desktop-skeleton, docs/character.md#behavior-rules).

mod common;

use common::{OUTSIDE_WADE, SEED, digit, ms};
use embedded_graphics::geometry::Point;
use std::time::Duration;

use wade_core::app::{
    FRAME, HUBRIS_ONE_IN, HUFF_POKES, POKE_GAP, PROUD_DURATION, PROUD_TAPS, STALL_LIMIT,
};
use wade_core::character::{
    BLINK_DURATION, BLINK_INTERVAL_MAX, BLINK_INTERVAL_MIN, DOUBLE_BLINK_GAP, Expression,
};
use wade_core::harness::Harness;
use wade_core::layout::WADE_CENTER;
use wade_core::view::EyeStyle;
use wade_core::{Event, Instant, Key, TouchPhase};

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
fn number_keys_hold_each_expression() {
    for (n, expression) in (0u8..).zip(Expression::ALL) {
        let mut h = Harness::new(SEED);
        h.key(ms(1_000), digit(n));
        assert_eq!(h.buddy().expression, expression);
        h.run_until(ms(60_000));
        assert_eq!(h.buddy().expression, expression, "{expression:?} expired");
        assert!(!h.buddy().asleep, "{expression:?} fell asleep");
    }
}

#[test]
fn p_switches_between_pupils_and_plain_eyes() {
    let mut h = Harness::new(SEED);
    assert_eq!(h.buddy().eye_style, EyeStyle::Pupils);
    h.key(ms(1_000), Key::P);
    assert_eq!(h.buddy().eye_style, EyeStyle::Plain);
    h.key(ms(2_000), Key::P);
    assert_eq!(h.buddy().eye_style, EyeStyle::Pupils);
}

#[test]
fn z_puts_him_to_sleep_and_a_tap_wakes_him_surprised() {
    let mut h = Harness::new(SEED);
    h.key(ms(1_000), Key::Z);
    assert!(h.buddy().asleep);
    assert_eq!(h.buddy().expression, Expression::Sleepy);

    h.run_until(ms(60_000));
    assert!(h.buddy().asleep);

    h.tap(ms(60_000), WADE_CENTER);
    assert!(!h.buddy().asleep);
    assert_eq!(h.buddy().expression, Expression::Surprised);
    h.run_until(ms(60_999));
    assert_eq!(h.buddy().expression, Expression::Surprised);
    h.run_until(ms(61_000));
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn the_power_button_puts_him_to_sleep_and_wakes_him_surprised() {
    let mut h = Harness::new(SEED);
    h.power_button(ms(1_000));
    assert!(h.buddy().asleep);
    assert_eq!(h.buddy().expression, Expression::Sleepy);

    h.power_button(ms(5_000));
    assert!(!h.buddy().asleep);
    assert_eq!(h.buddy().expression, Expression::Surprised);
    h.run_until(ms(5_999));
    assert_eq!(h.buddy().expression, Expression::Surprised);
    h.run_until(ms(6_000));
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn a_tap_wakes_him_after_the_power_button() {
    let mut h = Harness::new(SEED);
    h.power_button(ms(1_000));
    h.tap(ms(5_000), WADE_CENTER);
    assert!(!h.buddy().asleep);
    assert_eq!(h.buddy().expression, Expression::Surprised);
}

#[test]
fn a_number_key_wakes_him_into_that_expression() {
    let mut h = Harness::new(SEED);
    h.key(ms(1_000), Key::Z);
    h.key(ms(5_000), digit(1));
    assert!(!h.buddy().asleep);
    assert_eq!(h.buddy().expression, Expression::Happy);
    h.run_until(ms(30_000));
    assert_eq!(h.buddy().expression, Expression::Happy);
}

#[test]
fn tap_during_a_held_expression_is_happy_then_neutral() {
    let mut h = Harness::new(SEED);
    h.key(ms(1_000), digit(3));
    h.tap(ms(2_000), WADE_CENTER);
    assert_eq!(h.buddy().expression, Expression::Happy);
    h.run_until(ms(4_000));
    assert_eq!(h.buddy().expression, Expression::Neutral);
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
fn he_does_not_blink_while_asleep() {
    let mut h = Harness::new(SEED);
    h.key(ms(1_000), Key::Z);
    follow_deadlines(&mut h, ms(120_000), |h, t| {
        assert!(!h.buddy().blinking, "blinked at {t:?} while asleep");
    });
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
fn wakeups_stay_well_below_the_frame_rate() {
    // Animating without end would wake the device on every frame.
    let frame_rate = 1_000 / FRAME.as_millis();
    let limits = [
        (digit(0), 6),  // Neutral
        (digit(2), 16), // Sad: a falling tear every few seconds
        (digit(3), 16), // Angry: trembles every 100 ms
        (digit(6), 6),  // Thinking: dots every 420 ms
        (Key::Z, 10),   // asleep: breath and Z's every 150 ms
    ];
    for (key, limit) in limits {
        let mut h = Harness::new(SEED);
        h.key(ms(1_000), key);
        h.run_until(ms(5_000));
        let mut wakeups = 0;
        follow_deadlines(&mut h, ms(65_000), |_, _| wakeups += 1);
        let per_second = wakeups / 60;
        assert!(
            per_second <= limit,
            "{key:?}: {per_second} wakeups a second, over {limit} (frames are {frame_rate})"
        );
    }
}

#[test]
fn b_plays_each_beat_in_turn() {
    let mut h = Harness::new(SEED);
    let firsts = [
        Expression::Thinking,
        Expression::Flustered,
        Expression::Surprised,
        Expression::Sad,
        Expression::Thinking,
    ];
    for (i, first) in (0..).zip(firsts) {
        h.key(ms(1_000 + i * 100), Key::B);
        assert_eq!(h.buddy().expression, first, "press {i}");
    }
}

#[test]
fn b_wakes_him_into_the_beat() {
    let mut h = Harness::new(SEED);
    h.key(ms(1_000), Key::Z);
    h.run_until(ms(3_000));
    assert!(h.buddy().asleep);
    h.key(ms(3_000), Key::B);
    assert!(!h.buddy().asleep);
    assert_eq!(h.buddy().expression, Expression::Thinking);
}

/// Tap Wade `n` times, `gap` ms apart, from `from` ms.
fn taps(h: &mut Harness, from: u64, n: u64, gap: u64) {
    for i in 0..n {
        h.tap(ms(from + i * gap), WADE_CENTER);
    }
}

/// Deliberate taps, slower than pokes.
const STEADY: u64 = 300;
/// Pokes, faster than `POKE_GAP`.
const RAPID: u64 = 100;

#[test]
fn the_tap_speeds_straddle_the_poke_gap() {
    assert_eq!(POKE_GAP, Duration::from_millis(200));
    assert_eq!((PROUD_TAPS, HUFF_POKES), (5, 6));
}

#[test]
fn four_steady_taps_keep_him_happy() {
    let mut h = Harness::new(SEED);
    taps(&mut h, 1_000, 4, STEADY);
    assert_eq!(h.buddy().expression, Expression::Happy);
}

#[test]
fn a_fifth_steady_tap_makes_him_proud() {
    let mut h = Harness::new(SEED);
    assert_eq!(PROUD_DURATION, Duration::from_secs(3));
    taps(&mut h, 1_000, 5, STEADY);
    assert_eq!(h.buddy().expression, Expression::Proud);
    h.run_until(ms(5_199));
    assert_eq!(h.buddy().expression, Expression::Proud);
    h.run_until(ms(5_200));
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn taps_spread_out_never_make_him_proud() {
    let mut h = Harness::new(SEED);
    for i in 0..10 {
        h.tap(ms(1_000 + i * 1_100), WADE_CENTER);
        assert_eq!(h.buddy().expression, Expression::Happy, "tap {i}");
    }
}

#[test]
fn a_steady_burst_keeps_him_proud_and_never_backfires() {
    for seed in 0..40 {
        let mut h = Harness::new(seed);
        taps(&mut h, 1_000, 7, STEADY);
        assert_eq!(h.buddy().expression, Expression::Proud, "seed {seed}");
    }
}

#[test]
fn poking_goes_straight_from_happy_to_a_huff() {
    let mut h = Harness::new(SEED);
    let mut seen = Vec::new();
    for i in 0..7 {
        h.tap(ms(1_000 + i * RAPID), WADE_CENTER);
        seen.push(h.buddy().expression);
    }
    let mut expected = [Expression::Happy; 7];
    expected[6] = Expression::Surprised;
    assert_eq!(seen, expected);
    h.run_until(ms(2_200));
    assert_eq!(h.buddy().expression, Expression::Angry);
}

#[test]
fn a_huff_ignores_taps_and_sours_his_mood() {
    let mut h = Harness::new(SEED);
    // Four steady taps leave him Pleased.
    taps(&mut h, 1_000, 4, STEADY);
    h.run_until(ms(6_000));
    assert!(h.buddy().pose.lower_lid > 0.05, "not Pleased");
    taps(&mut h, 6_000, 7, RAPID);
    h.run_until(ms(7_200));
    assert_eq!(h.buddy().expression, Expression::Angry);
    // A steady tap during the huff gets no reaction.
    h.tap(ms(7_300), WADE_CENTER);
    assert_eq!(h.buddy().expression, Expression::Angry);
    h.run_until(ms(10_000));
    assert_eq!(h.buddy().expression, Expression::Neutral);
    assert!(h.buddy().pose.lower_lid.abs() < 1e-6, "still Pleased");
}

#[test]
fn endless_poking_keeps_him_huffing_until_it_stops() {
    let mut h = Harness::new(SEED);
    let mut angry = 0;
    for i in 0..100 {
        h.tap(ms(1_000 + i * RAPID), WADE_CENTER);
        assert_ne!(h.buddy().expression, Expression::Proud, "tap {i}");
        angry += usize::from(h.buddy().expression == Expression::Angry);
    }
    assert!(angry > 60, "angry for only {angry} of 100 taps");
    h.run_until(ms(14_000));
    assert_eq!(h.buddy().expression, Expression::Neutral);
}

#[test]
fn a_fresh_tap_while_proud_sometimes_backfires() {
    let (mut backfired, mut restarted) = (0, 0);
    for seed in 0..200 {
        let mut h = Harness::new(seed);
        taps(&mut h, 1_000, 5, STEADY);
        // More than a burst window after the last tap, while still Proud.
        h.tap(ms(4_300), WADE_CENTER);
        match h.buddy().expression {
            Expression::Flustered => backfired += 1,
            Expression::Proud => restarted += 1,
            other => panic!("seed {seed}: {other:?}"),
        }
    }
    assert_eq!(HUBRIS_ONE_IN, 4);
    assert!(
        (30..=70).contains(&backfired) && backfired + restarted == 200,
        "{backfired} backfired and {restarted} restarted Proud, of 200"
    );
}
