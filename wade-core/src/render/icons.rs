//! Icons drawn from primitives. Screens show icons, never words (D26).

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle, RoundedRectangle},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
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
