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
    widgets::paged_tiles(
        &layout::LAUNCHER_TILES,
        view.page,
        view.pressed,
        |tile, _| icon(tile),
        target,
    )
}

/// Each tile shows its app's title icon.
const fn icon(tile: Target) -> Icon {
    match tile {
        Target::Tile(Tile::Timer) => Icon::Stopwatch,
        // Settings, the only other tile.
        _ => Icon::Gear,
    }
}
