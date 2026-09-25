//! Settings, the Settings screen, and how settings are stored
//! (docs/roadmap.md#m5-settings, docs/ui.md#settings).

mod common;

use std::collections::HashSet;
use std::time::Duration;

use common::{SEED, button, every_settings, home, ms, open, setting};
use proptest::prelude::*;
use wade_core::app::Screen;
use wade_core::harness::Harness;
use wade_core::layout::{Target, Tile};
use wade_core::settings::{ENCODED_LEN, SettingsButton};
use wade_core::timer::{MIN_SET, TimerButton, TimerPhase, TimerState};
use wade_core::view::{ColorMode, EyeStyle};
use wade_core::{Effect, Key, Settings, TouchPhase};

#[test]
fn every_value_survives_encoding() {
    let all: Vec<_> = every_settings().collect();
    assert_eq!(all.len(), 2 * 2 * 2 * 99);
    for settings in all {
        assert_eq!(Settings::decode(&settings.encode()), settings);
    }
}

proptest! {
    #[test]
    fn any_bytes_decode_to_the_defaults_or_to_what_they_encode(
        bytes in prop_oneof![
            prop::collection::vec(any::<u8>(), 0..=2 * ENCODED_LEN),
            // Near misses: the right length, each byte at or just past its valid range.
            (0u8..3, 0u8..3, 0u8..3, 0u8..3, 0u8..=100)
                .prop_map(|(v, e, c, h, m)| vec![v, e, c, h, m]),
        ],
    ) {
        if let Some(settings) = Settings::try_decode(&bytes) {
            prop_assert_eq!(&settings.encode()[..], &bytes[..]);
        } else {
            prop_assert_eq!(Settings::decode(&bytes), Settings::DEFAULT);
        }
    }
}

#[test]
fn the_app_starts_with_the_settings_it_is_given() {
    let settings = Settings::DEFAULT
        .with_eye_style(EyeStyle::Plain)
        .with_timer(Duration::from_mins(12))
        .expect("12 minutes");
    let h = Harness::with_settings(SEED, settings);
    assert_eq!(h.app.settings(), settings);
    assert_eq!(h.buddy().eye_style, EyeStyle::Plain);
    assert_eq!(
        h.app.timer_state(),
        TimerState::Ready {
            set: Duration::from_mins(12)
        }
    );
}

#[test]
fn p_changes_the_eye_style_setting() {
    let mut h = Harness::new(SEED);
    h.key(ms(1_000), Key::P);
    assert_eq!(h.app.settings().eye_style(), EyeStyle::Plain);
    h.key(ms(2_000), Key::P);
    assert_eq!(h.app.settings(), Settings::DEFAULT);
}

#[test]
fn the_state_hash_covers_every_setting() {
    let hashes: HashSet<_> = every_settings()
        .map(|settings| Harness::with_settings(SEED, settings).state_hash())
        .collect();
    assert_eq!(hashes.len(), every_settings().count());
}

#[test]
fn eye_style_and_colors_change_wade_at_once() {
    let mut h = Harness::new(SEED);
    open(&mut h, ms(1_000), Tile::Settings);
    h.tap(ms(2_000), setting(SettingsButton::EyeStyle));
    h.tap(ms(2_500), setting(SettingsButton::Color));
    assert_eq!(h.settings().settings.eye_style(), EyeStyle::Plain);
    assert_eq!(h.settings().settings.color(), ColorMode::Mono);
    home(&mut h, ms(3_000));
    assert_eq!(h.buddy().eye_style, EyeStyle::Plain);
    assert_eq!(h.buddy().color, ColorMode::Mono);
}

/// Run a 1:00 timer from Buddy, started at 2 s, so it finishes at 62 s, and
/// then let every repeat fall due.
fn run_a_one_minute_timer(h: &mut Harness) {
    open(h, ms(1_000), Tile::Timer);
    for step in 1..=4 {
        h.tap(ms(1_000 + step * 100), button(TimerButton::Minus));
    }
    h.tap(ms(2_000), button(TimerButton::Start));
    home(h, ms(3_000));
    h.run_until(ms(62_000));
    assert_eq!(h.app.screen(), Screen::Timer);
    assert_eq!(h.timer().phase, TimerPhase::Done);
    h.run_until(ms(100_000));
}

#[test]
fn chime_off_silences_the_first_chime_and_every_repeat() {
    let mut h = Harness::new(SEED);
    open(&mut h, ms(500), Tile::Settings);
    h.tap(ms(600), setting(SettingsButton::Chime));
    assert!(!h.settings().settings.chime());
    home(&mut h, ms(700));
    run_a_one_minute_timer(&mut h);
    assert!(!h.effects.contains(&Effect::Chime));

    let mut chiming = Harness::new(SEED);
    run_a_one_minute_timer(&mut chiming);
    let chimes = chiming.effects.iter().filter(|&&e| e == Effect::Chime);
    assert_eq!(chimes.count(), 10);
}

#[test]
fn a_silent_timer_still_wakes_wade() {
    let silent_minute = Settings::DEFAULT
        .with_chime(false)
        .with_timer(MIN_SET)
        .expect("1 minute");
    let mut h = Harness::with_settings(SEED, silent_minute);
    h.key(ms(500), Key::Z);
    open(&mut h, ms(1_000), Tile::Timer);
    h.tap(ms(2_000), button(TimerButton::Start));
    home(&mut h, ms(3_000));
    h.run_until(ms(61_999));
    assert!(h.wade_asleep());
    h.run_until(ms(62_000));
    assert!(!h.wade_asleep());
    assert!(!h.effects.contains(&Effect::Chime));
}

#[test]
fn the_timer_remembers_the_duration_last_set() {
    let mut h = Harness::new(SEED);
    open(&mut h, ms(1_000), Tile::Timer);
    h.tap(ms(2_000), button(TimerButton::Plus));
    h.tap(ms(2_100), button(TimerButton::Plus));
    let seven = Duration::from_mins(7);
    assert_eq!(h.app.settings().timer(), seven);
    // Running it and resetting it keep the duration.
    h.tap(ms(3_000), button(TimerButton::Start));
    h.tap(ms(4_000), button(TimerButton::Pause));
    h.tap(ms(5_000), button(TimerButton::Reset));
    assert_eq!(h.app.settings().timer(), seven);
    let next_start = Harness::with_settings(SEED, h.app.settings());
    assert_eq!(
        next_start.app.timer_state(),
        TimerState::Ready { set: seven }
    );
}

#[test]
fn a_settings_button_shows_pressed_while_the_touch_stays_on_it() {
    let mut h = Harness::new(SEED);
    open(&mut h, ms(1_000), Tile::Settings);
    let chime = setting(SettingsButton::Chime);
    h.touch(ms(2_000), TouchPhase::Down, chime);
    assert_eq!(
        h.settings().pressed,
        Some(Target::Settings(SettingsButton::Chime))
    );
    h.touch(ms(2_050), TouchPhase::Move, setting(SettingsButton::Color));
    assert_eq!(h.settings().pressed, None);
    h.touch(ms(2_100), TouchPhase::Up, chime);
    assert_eq!(h.app.settings(), Settings::DEFAULT);
}
