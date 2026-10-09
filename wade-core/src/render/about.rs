//! Read-only build identity, the text exception to the icon-based controls.

use core::fmt::Write;

use embedded_graphics::{
    mono_font::{
        MonoTextStyle,
        ascii::{FONT_8X13, FONT_10X20},
    },
    pixelcolor::Rgb565,
    prelude::*,
    text::Text,
};

use crate::layout::Target;
use crate::render::{icons::Icon, palette, widgets};
use crate::view::AboutView;

pub fn draw<D>(view: AboutView, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    widgets::back_button(view.pressed == Some(Target::Back), target)?;
    widgets::title(Icon::Info, target)?;
    Text::new(
        "Wade",
        Point::new(64, 73),
        MonoTextStyle::new(&FONT_10X20, palette::EYE),
    )
    .draw(target)?;
    let style = MonoTextStyle::new(&FONT_8X13, palette::EYE);
    let mut build = heapless::String::<48>::new();
    let _ = write!(build, "Build {}", view.info.revision);
    for (y, text) in [
        (102, view.info.version),
        (128, view.info.board),
        (154, build.as_str()),
    ] {
        Text::new(text, Point::new(64, y), style).draw(target)?;
    }
    let status = match (view.info.development, view.info.dirty) {
        (_, true) => "Development / local changes",
        (true, false) => "Development build",
        (false, false) => "Release build",
    };
    Text::new(
        status,
        Point::new(48, 188),
        MonoTextStyle::new(&FONT_8X13, palette::DIM),
    )
    .draw(target)?;
    Ok(())
}
