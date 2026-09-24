//! Helpers shared by the integration tests.

#![allow(dead_code)] // Each test crate uses a different subset.

pub mod framebuffer;

use embedded_graphics::geometry::Point;
use wade_core::Instant;

/// The fixed seed for behavior tests.
pub const SEED: u64 = 0x5EED_F00D;

pub fn ms(ms: u64) -> Instant {
    Instant::from_millis(ms)
}

/// A point on the Buddy screen well away from Wade and the apps button.
pub const OUTSIDE_WADE: Point = Point::new(10, 10);
