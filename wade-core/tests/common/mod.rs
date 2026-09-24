//! Helpers shared by the integration tests.

#![allow(dead_code, reason = "each test crate uses a different subset")]

pub mod framebuffer;

use embedded_graphics::geometry::Point;
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
