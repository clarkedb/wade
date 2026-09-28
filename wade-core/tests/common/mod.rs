//! Helpers shared by the integration tests.

#![allow(dead_code, reason = "each test crate uses a different subset")]

pub mod framebuffer;

use embedded_graphics::geometry::Point;
use wade_core::harness::Harness;
use wade_core::layout::{self, Tile};
use wade_core::timer::TimerButton;
use wade_core::{Digit, Instant, Key};

/// The fixed seed for behavior tests.
pub const SEED: u64 = 0x5EED_F00D;

pub fn ms(ms: u64) -> Instant {
    Instant::from_millis(ms)
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

/// The middle of `tile` on the Launcher.
pub fn tile(tile: Tile) -> Point {
    let (_, area) = layout::TILES
        .into_iter()
        .find(|&(t, _)| t == tile)
        .expect("every app has a tile");
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
        TimerButton::Plus => 2,
    };
    layout::TIMER_ROW[slot].center()
}
