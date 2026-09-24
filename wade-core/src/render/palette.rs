//! Every color, defined once. Check them on the device: the panel renders Rgb565
//! differently from a desktop monitor.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::RgbColor;

pub const BACKGROUND: Rgb565 = Rgb565::BLACK;
/// Eyes and marks: the cool white of a small OLED.
pub const EYE: Rgb565 = Rgb565::new(27, 58, 31);
pub const BLUSH: Rgb565 = Rgb565::new(31, 30, 22);
/// Tears and sweat.
pub const WATER: Rgb565 = Rgb565::new(12, 46, 31);
pub const SPARKLE: Rgb565 = Rgb565::new(31, 56, 8);
