//! The Timer screen (docs/ui.md#timer).

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use crate::layout::{self, Target};
use crate::render::icons::{self, Icon};
use crate::render::{palette, widgets};
use crate::view::TimerView;

pub fn draw<D>(view: TimerView, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    widgets::back_button(view.pressed == Some(Target::Back), target)?;
    icons::draw(Icon::Stopwatch, layout::TITLE, palette::EYE, target)
}
