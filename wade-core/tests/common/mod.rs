//! Helpers shared by the integration tests.

#![allow(dead_code, reason = "each test crate uses a different subset")]

pub mod framebuffer;

use std::time::Duration;

use embedded_graphics::geometry::Point;
use wade_core::app::POKE_GAP;
use wade_core::harness::Harness;
use wade_core::layout::{self, SettingsTile, Tile, WADE_CENTER};
use wade_core::settings::{Brightness, SettingsButton, Volume};
use wade_core::timer::TimerButton;
use wade_core::view::{ColorMode, EyeStyle};
use wade_core::{Digit, Instant, Key, Settings};

/// The fixed seed for behavior tests.
pub const SEED: u64 = 0x5EED_F00D;

pub fn ms(ms: u64) -> Instant {
    Instant::from_millis(ms)
}

/// A gap between taps on Wade that makes them deliberate, and one that makes
/// them pokes (docs/character.md#taps-on-wade).
pub const STEADY: Duration = Duration::from_millis(300);
pub const RAPID: Duration = Duration::from_millis(100);
const _: () =
    assert!(RAPID.as_millis() < POKE_GAP.as_millis() && POKE_GAP.as_millis() <= STEADY.as_millis());

/// Tap Wade `n` times, `gap` apart, from `from`. Returns the last tap's time.
pub fn tap_wade(h: &mut Harness, from: Instant, n: u32, gap: Duration) -> Instant {
    let mut at = from;
    for i in 0..n {
        at = from + gap * i;
        h.tap(at, WADE_CENTER);
    }
    at
}

/// A point on the Buddy screen well away from Wade and the apps button.
pub const OUTSIDE_WADE: Point = Point::new(10, 10);

/// The number key `n`, 0–9.
pub fn digit(n: u8) -> Key {
    Key::Digit(Digit::new(n).expect("a digit is 0–9"))
}

/// The middle of the apps button on the Buddy screen.
pub fn apps() -> Point {
    layout::APPS.center()
}

/// The middle of the back button on app screens.
pub fn back() -> Point {
    layout::BACK.center()
}

/// The middle of `tile` on the Launcher, which must show its page.
pub fn tile(tile: Tile) -> Point {
    let (_, area) =
        layout::find_tile(&layout::LAUNCHER_TILES, &tile).expect("every app has a tile");
    area.center()
}

/// The middle of `button` on the Settings screen, which must show its page.
pub fn setting(button: SettingsButton) -> Point {
    let (_, area) = layout::find_tile(&layout::SETTINGS_TILES, &SettingsTile::Toggle(button))
        .expect("every setting has a tile");
    area.center()
}

/// The middle of the information button on the Settings screen, which must
/// show its page.
pub fn about() -> Point {
    let (_, area) = layout::find_tile(&layout::SETTINGS_TILES, &SettingsTile::About)
        .expect("Settings has an information tile");
    area.center()
}

/// From Buddy at `t`, open the Launcher and then `app`.
pub fn open(h: &mut Harness, t: Instant, app: Tile) {
    h.tap(t, apps());
    h.tap(t, tile(app));
}

/// From an app at `t`, go back to the Launcher and then to Buddy.
pub fn home(h: &mut Harness, t: Instant) {
    h.tap(t, back());
    h.tap(t, back());
}

/// The middle of `button` in the timer's row. Each button has one slot.
pub fn button(button: TimerButton) -> Point {
    let slot = match button {
        TimerButton::Minus | TimerButton::Reset => 0,
        TimerButton::Start | TimerButton::Pause | TimerButton::Resume | TimerButton::Dismiss => 1,
        TimerButton::Plus | TimerButton::Watch => 2,
    };
    layout::TIMER_ROW[slot].center()
}

/// Every possible value of the settings.
pub fn every_settings() -> impl Iterator<Item = Settings> {
    [EyeStyle::Pupils, EyeStyle::Plain]
        .into_iter()
        .flat_map(|eye_style| {
            [ColorMode::Color, ColorMode::Mono]
                .into_iter()
                .map(move |color| (eye_style, color))
        })
        .flat_map(|(eye_style, color)| Volume::ALL.map(|volume| (eye_style, color, volume)))
        .flat_map(|(eye_style, color, volume)| {
            Brightness::ALL.map(|brightness| (eye_style, color, volume, brightness))
        })
        .flat_map(|(eye_style, color, volume, brightness)| {
            (1..=99).map(move |minutes| {
                Settings::DEFAULT
                    .with_eye_style(eye_style)
                    .with_color(color)
                    .with_volume(volume)
                    .with_brightness(brightness)
                    .with_timer(Duration::from_mins(minutes))
                    .expect("1 to 99 minutes is a valid timer")
            })
        })
}
