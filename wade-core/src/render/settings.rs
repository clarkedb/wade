//! The Settings screen (docs/ui.md#settings).

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use crate::layout::Target;
use crate::render::icons::Icon;
use crate::render::widgets;
use crate::view::SettingsView;

pub fn draw<D>(view: SettingsView, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    widgets::back_button(view.pressed == Some(Target::Back), target)?;
    widgets::title(Icon::Gear, target)
}
