mod common;

use common::{SEED, back, button, ms, open, setting};
use wade_core::app::Screen;
use wade_core::harness::{Harness, state_hash};
use wade_core::layout::{self, Tile};
use wade_core::settings::SettingsButton;
use wade_core::timer::{MIN_SET, TimerButton};
use wade_core::view::SettingsPage;
use wade_core::{BuildInfo, Effect, Settings, TouchPhase, View};

const INFO: BuildInfo = BuildInfo {
    version: "0.2.0",
    board: "CoreS3 Lite",
    revision: "a1b2c3d",
    development: true,
    dirty: true,
};

#[test]
fn about_returns_to_settings_and_saves_pending_changes_once() {
    let mut h = Harness::new(SEED);
    h.app = h.app.with_build_info(INFO);
    open(&mut h, ms(1_000), Tile::Settings);
    h.tap(ms(1_010), setting(SettingsButton::Chime));
    h.effects.clear();
    h.tap(ms(1_020), layout::SETTINGS_NEXT.center());
    h.tap(ms(1_021), layout::ABOUT.center());
    let View::About(view) = h.app.view() else {
        panic!("About is shown")
    };
    assert_eq!(view.info, INFO);
    assert_eq!(
        h.effects,
        [Effect::SaveSettings(Settings::DEFAULT.with_chime(false))]
    );
    h.effects.clear();
    assert_eq!(h.app.next_deadline(), None);
    h.tap(ms(1_030), layout::ABOUT.center());
    assert_eq!(h.app.screen(), Screen::About, "metadata has no controls");
    h.tap(ms(1_040), back());
    assert_eq!(h.app.screen(), Screen::Settings);
    assert_eq!(h.settings().page, SettingsPage::Information);
    assert!(!h.app.settings().chime());
    assert!(h.effects.is_empty());
}

#[test]
fn build_identity_never_changes_recording_hashes() {
    let mut h = Harness::new(SEED);
    for point in [
        layout::APPS.center(),
        common::tile(Tile::Settings),
        layout::SETTINGS_NEXT.center(),
        layout::ABOUT.center(),
    ] {
        h.tap(ms(1_000), point);
        assert_eq!(
            state_hash(&h.app),
            state_hash(&h.app.clone().with_build_info(INFO))
        );
    }
}

#[test]
fn a_timer_finishing_on_about_cancels_the_back_press() {
    let mut h = Harness::with_settings(SEED, Settings::DEFAULT.with_timer(MIN_SET).unwrap());
    open(&mut h, ms(1_000), Tile::Timer);
    h.tap(ms(1_010), button(TimerButton::Start));
    h.tap(ms(1_020), back());
    h.tap(ms(1_030), common::tile(Tile::Settings));
    h.tap(ms(1_040), layout::SETTINGS_NEXT.center());
    h.tap(ms(1_050), layout::ABOUT.center());
    h.touch(ms(61_000), TouchPhase::Down, back());
    h.run_until(ms(61_010));
    assert_eq!(h.app.screen(), Screen::Timer);
    h.touch(ms(61_520), TouchPhase::Up, back());
    assert_eq!(h.app.screen(), Screen::Timer);
}
