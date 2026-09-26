//! Drawing. A pure function of a `View` (D9): every call draws a complete frame.

mod digits;
pub mod face;
mod icons;
mod launcher;
pub mod palette;
mod settings;
mod timer;
mod widgets;

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
        View::Buddy(buddy) => {
            face::draw(&buddy.pose, buddy.eye_style, buddy.color, target)?;
            widgets::apps_button(buddy.apps_pressed, target)
        }
        View::Launcher(launcher) => launcher::draw(*launcher, target),
        View::Timer(timer) => timer::draw(*timer, target),
        View::Settings(settings) => settings::draw(*settings, target),
    }
}
