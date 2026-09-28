//! The Launcher: a grid of app tiles (docs/ui.md#launcher).

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use crate::layout::{self, Target, Tile};
use crate::render::icons::Icon;
use crate::render::widgets;
use crate::view::LauncherView;

pub fn draw<D>(view: LauncherView, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    widgets::back_button(view.pressed == Some(Target::Back), target)?;
    widgets::title(Icon::Apps, target)?;
    for (tile, area) in layout::TILES {
        let pressed = view.pressed == Some(Target::Tile(tile));
        let scale = widgets::TILE_ICON_SCALE;
        widgets::button(area, icon(tile), scale, true, pressed, target)?;
    }
    Ok(())
}

/// Each tile shows its app's title icon.
const fn icon(tile: Tile) -> Icon {
    match tile {
        Tile::Timer => Icon::Stopwatch,
        Tile::Settings => Icon::Gear,
    }
}
