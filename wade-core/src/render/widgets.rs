//! The navigation buttons shared by the screens.

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle},
};

use crate::layout;
use crate::render::palette;

/// The apps button: a small, low-contrast grid of four squares.
pub fn apps_button<D>(pressed: bool, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let color = nav_button(layout::APPS, pressed, palette::DIM, target)?;
    let origin = layout::APPS.top_left;
    for (x, y) in [(14, 14), (26, 14), (14, 26), (26, 26)] {
        RoundedRectangle::with_equal_corners(
            Rectangle::new(origin + Point::new(x, y), Size::new(8, 8)),
            Size::new(2, 2),
        )
        .into_styled(PrimitiveStyle::with_fill(color))
        .draw(target)?;
    }
    Ok(())
}

/// The back button: a small, low-contrast chevron pointing left.
pub fn back_button<D>(pressed: bool, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let color = nav_button(layout::BACK, pressed, palette::DIM, target)?;
    // Each arm steps one pixel across per pixel down, a true 45°, so the
    // staircase is even at this size where a stroked line comes out ragged.
    let tip = layout::BACK.top_left + Point::new(19, 22);
    let step = |dx: i32, dy: i32| {
        Rectangle::new(tip + Point::new(dx, dy), Size::new(3, 3))
            .into_styled(PrimitiveStyle::with_fill(color))
    };
    for k in 0..8 {
        step(k, -k).draw(target)?;
        step(k, k).draw(target)?;
    }
    Ok(())
}

/// Draw a navigation button's pressed look, if pressed, and return the color
/// for its icon. Pressed, the button inverts: a disc behind a dark icon.
fn nav_button<D>(
    area: Rectangle,
    pressed: bool,
    color: Rgb565,
    target: &mut D,
) -> Result<Rgb565, D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    if !pressed {
        return Ok(color);
    }
    Circle::new(area.top_left + Point::new(4, 4), 40)
        .into_styled(PrimitiveStyle::with_fill(palette::EYE))
        .draw(target)?;
    Ok(palette::BACKGROUND)
}
