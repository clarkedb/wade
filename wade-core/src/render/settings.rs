//! The Settings screen (docs/ui.md#settings).

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use crate::layout::{self, SettingsTile, Target};
use crate::render::icons::Icon;
use crate::render::widgets;
use crate::settings::{Settings, SettingsButton};
use crate::view::{ColorMode, EyeStyle, SettingsView};

pub fn draw<D>(view: SettingsView, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    widgets::back_button(view.pressed == Some(Target::Back), target)?;
    widgets::title(Icon::Gear, target)?;
    widgets::paged_tiles(
        &layout::SETTINGS_TILES,
        view.page,
        view.pressed,
        |tile, pressed| match tile {
            SettingsTile::Toggle(button) => icon(button, view.settings, pressed),
            SettingsTile::About => Icon::Info,
        },
        target,
    )
}

/// Each toggle shows its setting's current value.
fn icon(button: SettingsButton, settings: Settings, pressed: bool) -> Icon {
    match button {
        SettingsButton::EyeStyle => match settings.eye_style() {
            EyeStyle::Pupils => Icon::Pupils,
            EyeStyle::Plain => Icon::PlainEyes,
        },
        // Pressed, plain dots stand in, since the accents' colors do not invert.
        SettingsButton::Color => match settings.color() {
            ColorMode::Color if !pressed => Icon::ColorDots,
            ColorMode::Color | ColorMode::Mono => Icon::Dots,
        },
        SettingsButton::Chime => {
            if settings.chime() {
                Icon::Bell
            } else {
                Icon::Muted
            }
        }
        SettingsButton::Brightness => Icon::Sun(settings.brightness()),
    }
}
