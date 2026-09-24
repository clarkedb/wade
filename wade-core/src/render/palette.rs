//! Every color, defined once. Check them on the device: the panel renders Rgb565
//! differently from a desktop monitor.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::RgbColor;

pub const BACKGROUND: Rgb565 = Rgb565::new(3, 8, 6);
pub const OUTLINE: Rgb565 = Rgb565::BLACK;
pub const SKIN: Rgb565 = Rgb565::new(30, 50, 10);
pub const EYE: Rgb565 = Rgb565::new(4, 8, 8);
pub const EYE_HIGHLIGHT: Rgb565 = Rgb565::WHITE;
