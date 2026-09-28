//! The Settings screen (docs/ui.md#settings).

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use crate::layout::{self, Target};
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
    for (button, area) in layout::SETTINGS_BUTTONS {
        let pressed = view.pressed == Some(Target::Settings(button));
        let icon = icon(button, view.settings, pressed);
        widgets::button(area, icon, widgets::TILE_ICON_SCALE, true, pressed, target)?;
    }
    Ok(())
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
    }
}
