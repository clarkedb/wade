//! Drawing. A pure function of a `View` (D9): every call draws a complete frame.

mod face;
pub mod palette;

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use crate::view::View;

/// Draw a complete frame for `view` into `target`.
///
/// # Errors
///
/// Returns the first error from `target`.
pub fn draw<D>(view: &View, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    target.clear(palette::BACKGROUND)?;
    match view {
        View::Buddy(buddy) => face::draw(&buddy.pose, buddy.eye_style, target),
    }
}
