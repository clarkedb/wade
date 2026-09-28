//! Buttons and titles shared by the screens.

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StrokeAlignment,
    },
};

use crate::layout;
use crate::render::icons::{self, Colors, Icon};
use crate::render::palette;

/// Tiles draw their icons at twice the size of a button's.
pub const TILE_ICON_SCALE: u32 = 2;

/// Corner radius and outline width of buttons.
const BUTTON_RADIUS: u32 = 12;
const BUTTON_LINE: u32 = 3;

/// A button showing `icon` at `scale`: outlined, filled while pressed, and
/// dimmed while it ignores taps.
pub fn button<D>(
    area: Rectangle,
    icon: Icon,
    scale: u32,
    enabled: bool,
    pressed: bool,
    target: &mut D,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let color = if enabled { palette::EYE } else { palette::DIM };
    let mut style = PrimitiveStyleBuilder::new()
        .stroke_color(color)
        .stroke_width(BUTTON_LINE)
        .stroke_alignment(StrokeAlignment::Inside);
    if pressed {
        style = style.fill_color(color);
    }
    RoundedRectangle::with_equal_corners(area, Size::new_equal(BUTTON_RADIUS))
        .into_styled(style.build())
        .draw(target)?;

    let colors = if pressed {
        Colors {
            ink: palette::BACKGROUND,
            paper: color,
        }
    } else {
        Colors {
            ink: color,
            paper: palette::BACKGROUND,
        }
    };
    icons::draw(icon, area, scale, colors, target)
}

/// An app screen's title: its icon in the top-right corner, which takes no touches.
pub fn title<D>(icon: Icon, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let colors = Colors {
        ink: palette::EYE,
        paper: palette::BACKGROUND,
    };
    icons::draw(icon, layout::TITLE, 1, colors, target)
}

/// The apps button: a small, low-contrast grid of four squares.
pub fn apps_button<D>(pressed: bool, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let colors = nav_button(layout::APPS, pressed, palette::DIM, target)?;
    icons::draw(Icon::Apps, layout::APPS, 1, colors, target)
}

/// The back button: a small, low-contrast chevron pointing left.
pub fn back_button<D>(pressed: bool, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let colors = nav_button(layout::BACK, pressed, palette::DIM, target)?;
    // Each arm steps one pixel across per pixel down, a true 45°, so the
    // staircase is even at this size where a stroked line comes out ragged.
    let tip = layout::BACK.top_left + Point::new(19, 22);
    let step = |dx: i32, dy: i32| {
        Rectangle::new(tip + Point::new(dx, dy), Size::new(3, 3))
            .into_styled(PrimitiveStyle::with_fill(colors.ink))
    };
    for k in 0..8 {
        step(k, -k).draw(target)?;
        step(k, k).draw(target)?;
    }
    Ok(())
}

/// Draw a navigation button's pressed look, if pressed, and return the colors
/// for its icon. Pressed, the button inverts: a disc behind a dark icon.
fn nav_button<D>(
    area: Rectangle,
    pressed: bool,
    color: Rgb565,
    target: &mut D,
) -> Result<Colors, D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    if !pressed {
        return Ok(Colors {
            ink: color,
            paper: palette::BACKGROUND,
        });
    }
    Circle::new(area.top_left + Point::new(4, 4), 40)
        .into_styled(PrimitiveStyle::with_fill(palette::EYE))
        .draw(target)?;
    Ok(Colors {
        ink: palette::BACKGROUND,
        paper: palette::EYE,
    })
}
