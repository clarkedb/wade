//! Settings and how they are stored (docs/roadmap.md#m5-settings).

mod common;

use std::collections::HashSet;
use std::time::Duration;

use common::{SEED, every_settings, ms};
use proptest::prelude::*;
use wade_core::harness::Harness;
use wade_core::settings::ENCODED_LEN;
use wade_core::timer::TimerState;
use wade_core::view::EyeStyle;
use wade_core::{Key, Settings};

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
