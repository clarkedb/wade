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
}

/// Draw `icon` in `color`, centered in `area`.
pub fn draw<T>(icon: Icon, area: Rectangle, color: Rgb565, target: &mut T) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    // The pixel just past the middle, so shapes of even size offset by half
    // their size sit exactly centered.
    let m = area.top_left + area.size / 2;
    let fill = PrimitiveStyle::with_fill(color);
    let bar = |x, y, w, h| {
        RoundedRectangle::with_equal_corners(
            Rectangle::new(m + Point::new(x, y), Size::new(w, h)),
            Size::new_equal(3),
        )
        .into_styled(fill)
    };
    match icon {
        Icon::Minus => bar(-12, -3, 24, 6).draw(target),
        Icon::Plus => {
            bar(-12, -3, 24, 6).draw(target)?;
            bar(-3, -12, 6, 24).draw(target)
        }
        Icon::Play => Triangle::new(
            m + Point::new(-8, -12),
            m + Point::new(-8, 11),
            m + Point::new(10, 0),
        )
        .into_styled(fill)
        .draw(target),
        Icon::Pause => {
            bar(-10, -12, 7, 24).draw(target)?;
            bar(3, -12, 7, 24).draw(target)
        }
        Icon::Stop => bar(-10, -10, 20, 20).draw(target),
        Icon::Check => Polyline::new(&[
            m + Point::new(-13, 0),
            m + Point::new(-4, 9),
            m + Point::new(13, -10),
        ])
        .into_styled(PrimitiveStyle::with_stroke(color, 6))
        .draw(target),
        Icon::Stopwatch => {
            let stroke = PrimitiveStyle::with_stroke(color, 3);
            Circle::new(m + Point::new(-12, -10), 24)
                .into_styled(stroke)
                .draw(target)?;
            bar(-4, -17, 8, 4).draw(target)?;
            Rectangle::new(m + Point::new(-1, -13), Size::new(2, 3))
                .into_styled(fill)
                .draw(target)?;
            Line::new(m + Point::new(0, 2), m + Point::new(5, -4))
                .into_styled(stroke)
                .draw(target)
        }
    }
}
