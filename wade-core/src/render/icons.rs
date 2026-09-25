//! Icons drawn from primitives. Screens show icons, never words (D26).

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Line, Polyline, PrimitiveStyle, Rectangle, RoundedRectangle, Triangle},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Minus,
    Plus,
    Play,
    Pause,
    Stop,
    Check,
    Stopwatch,
    /// Four squares, the apps button's mark.
    Apps,
    Gear,
}

/// The colors an icon is drawn in: `ink` for its shapes, and `paper`, the
/// color behind it, for holes cut through them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colors {
    pub ink: Rgb565,
    pub paper: Rgb565,
}

/// Draw `icon` centered in `area`, `scale` times its natural size of about
/// 24 px.
pub fn draw<T>(
    icon: Icon,
    area: Rectangle,
    scale: u32,
    colors: Colors,
    target: &mut T,
) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    let s = scale.cast_signed();
    // The pixel just past the middle, so shapes of even size offset by half
    // their size sit exactly centered.
    let m = area.top_left + area.size / 2;
    let at = |x: i32, y: i32| m + Point::new(x * s, y * s);
    let len = |n: u32| n * scale;
    let fill = PrimitiveStyle::with_fill(colors.ink);
    let stroke = |width| PrimitiveStyle::with_stroke(colors.ink, len(width));
    let bar = |x, y, w, h| {
        RoundedRectangle::with_equal_corners(
            Rectangle::new(at(x, y), Size::new(len(w), len(h))),
            Size::new_equal(len(3)),
        )
        .into_styled(fill)
    };
    match icon {
        Icon::Minus => bar(-12, -3, 24, 6).draw(target),
        Icon::Plus => {
            bar(-12, -3, 24, 6).draw(target)?;
            bar(-3, -12, 6, 24).draw(target)
        }
        Icon::Play => Triangle::new(at(-8, -12), at(-8, 11), at(10, 0))
            .into_styled(fill)
            .draw(target),
        Icon::Pause => {
            bar(-10, -12, 7, 24).draw(target)?;
            bar(3, -12, 7, 24).draw(target)
        }
        Icon::Stop => bar(-10, -10, 20, 20).draw(target),
        Icon::Check => Polyline::new(&[at(-13, 0), at(-4, 9), at(13, -10)])
            .into_styled(stroke(6))
            .draw(target),
        Icon::Stopwatch => {
            Circle::new(at(-12, -10), len(24))
                .into_styled(stroke(3))
                .draw(target)?;
            bar(-4, -17, 8, 4).draw(target)?;
            Rectangle::new(at(-1, -13), Size::new(len(2), len(3)))
                .into_styled(fill)
                .draw(target)?;
            Line::new(at(0, 2), at(5, -4))
                .into_styled(stroke(3))
                .draw(target)
        }
        Icon::Apps => {
            for (x, y) in [(-10, -10), (2, -10), (-10, 2), (2, 2)] {
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(at(x, y), Size::new_equal(len(8))),
                    Size::new_equal(len(2)),
                )
                .into_styled(fill)
                .draw(target)?;
            }
            Ok(())
        }
        Icon::Gear => {
            // Eight teeth: four bars through the middle, the diagonals as
            // long as the others.
            for (x, y) in [(12, 0), (0, 12), (9, 9), (9, -9)] {
                Line::new(at(-x, -y), at(x, y))
                    .into_styled(stroke(6))
                    .draw(target)?;
            }
            Circle::with_center(m, len(18))
                .into_styled(fill)
                .draw(target)?;
            Circle::with_center(m, len(8))
                .into_styled(PrimitiveStyle::with_fill(colors.paper))
                .draw(target)
        }
    }
}
