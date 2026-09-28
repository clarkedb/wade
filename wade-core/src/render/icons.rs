//! Icons drawn from primitives. Screens show icons, never words (D26).

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{
        Circle, Line, Polyline, PrimitiveStyle, Rectangle, RoundedRectangle, Styled, Triangle,
    },
};

use crate::render::palette;

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
    /// A small pair of Wade's eyes, with pupils.
    Pupils,
    /// The same eyes, plain.
    PlainEyes,
    /// Three dots in the ink.
    Dots,
    /// Three dots in the accents' colors.
    ColorDots,
    Bell,
    /// A bell struck through.
    Muted,
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
    let pen = Pen {
        // The pixel just past the middle, so shapes of even size offset by
        // half their size sit exactly centered.
        middle: area.top_left + area.size / 2,
        scale,
        colors,
    };
    match icon {
        Icon::Minus => pen.bar(-12, -3, 24, 6).draw(target),
        Icon::Plus => {
            pen.bar(-12, -3, 24, 6).draw(target)?;
            pen.bar(-3, -12, 6, 24).draw(target)
        }
        Icon::Play => Triangle::new(pen.at(-8, -12), pen.at(-8, 11), pen.at(10, 0))
            .into_styled(pen.ink())
            .draw(target),
        Icon::Pause => {
            pen.bar(-10, -12, 7, 24).draw(target)?;
            pen.bar(3, -12, 7, 24).draw(target)
        }
        Icon::Stop => pen.bar(-10, -10, 20, 20).draw(target),
        Icon::Check => Polyline::new(&[pen.at(-13, 0), pen.at(-4, 9), pen.at(13, -10)])
            .into_styled(pen.stroke(6))
            .draw(target),
        Icon::Stopwatch => stopwatch(&pen, target),
        Icon::Apps => {
            for (x, y) in [(-10, -10), (2, -10), (-10, 2), (2, 2)] {
                pen.rounded(x, y, 8, 8, 2).draw(target)?;
            }
            Ok(())
        }
        Icon::Gear => gear(&pen, target),
        Icon::Pupils => eyes(&pen, true, target),
        Icon::PlainEyes => eyes(&pen, false, target),
        Icon::Dots => dots(&pen, [pen.colors.ink; 3], target),
        Icon::ColorDots => dots(
            &pen,
            [palette::BLUSH, palette::WATER, palette::SPARKLE],
            target,
        ),
        Icon::Bell => bell(&pen, target),
        Icon::Muted => {
            bell(&pen, target)?;
            // Edged in the paper color, so the slash stands clear of the bell.
            let slash = Line::new(pen.at(-12, -12), pen.at(12, 12));
            slash
                .into_styled(PrimitiveStyle::with_stroke(pen.colors.paper, pen.len(7)))
                .draw(target)?;
            slash.into_styled(pen.stroke(3)).draw(target)
        }
    }
}

/// Places an icon's shapes around its middle, at its scale and in its colors.
struct Pen {
    middle: Point,
    scale: u32,
    colors: Colors,
}

impl Pen {
    /// The point `(x, y)` natural-size pixels from the middle, scaled.
    fn at(&self, x: i32, y: i32) -> Point {
        let s = self.scale.cast_signed();
        self.middle + Point::new(x * s, y * s)
    }

    /// `n` natural-size pixels, scaled.
    const fn len(&self, n: u32) -> u32 {
        n * self.scale
    }

    fn ink(&self) -> PrimitiveStyle<Rgb565> {
        PrimitiveStyle::with_fill(self.colors.ink)
    }

    fn stroke(&self, width: u32) -> PrimitiveStyle<Rgb565> {
        PrimitiveStyle::with_stroke(self.colors.ink, self.len(width))
    }

    /// A filled rectangle from `(x, y)`, `width` by `height`, with corners of
    /// `radius`.
    fn rounded(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        radius: u32,
    ) -> Styled<RoundedRectangle, PrimitiveStyle<Rgb565>> {
        RoundedRectangle::with_equal_corners(
            Rectangle::new(self.at(x, y), Size::new(self.len(width), self.len(height))),
            Size::new_equal(self.len(radius)),
        )
        .into_styled(self.ink())
    }

    /// A bar with softly rounded corners.
    fn bar(
        &self,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    ) -> Styled<RoundedRectangle, PrimitiveStyle<Rgb565>> {
        self.rounded(x, y, w, h, 3)
    }

    /// A disc of diameter `d` centered on `(x, y)`, in `color`.
    fn disc(
        &self,
        x: i32,
        y: i32,
        d: u32,
        color: Rgb565,
    ) -> Styled<Circle, PrimitiveStyle<Rgb565>> {
        Circle::with_center(self.at(x, y), self.len(d))
            .into_styled(PrimitiveStyle::with_fill(color))
    }
}

fn stopwatch<T>(pen: &Pen, target: &mut T) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    Circle::new(pen.at(-12, -10), pen.len(24))
        .into_styled(pen.stroke(3))
        .draw(target)?;
    pen.bar(-4, -17, 8, 4).draw(target)?;
    Rectangle::new(pen.at(-1, -13), Size::new(pen.len(2), pen.len(3)))
        .into_styled(pen.ink())
        .draw(target)?;
    Line::new(pen.at(0, 2), pen.at(5, -4))
        .into_styled(pen.stroke(3))
        .draw(target)
}

fn gear<T>(pen: &Pen, target: &mut T) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    // Eight teeth: four bars through the middle, the diagonals as long as the
    // others.
    for (x, y) in [(12, 0), (0, 12), (9, 9), (9, -9)] {
        Line::new(pen.at(-x, -y), pen.at(x, y))
            .into_styled(pen.stroke(6))
            .draw(target)?;
    }
    pen.disc(0, 0, 18, pen.colors.ink).draw(target)?;
    pen.disc(0, 0, 8, pen.colors.paper).draw(target)
}

fn eyes<T>(pen: &Pen, pupils: bool, target: &mut T) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    for x in [-17, 3] {
        pen.rounded(x, -7, 15, 15, 5).draw(target)?;
        if pupils {
            pen.disc(x + 7, 0, 7, pen.colors.paper).draw(target)?;
        }
    }
    Ok(())
}

/// Three dots in a triangle, in `colors`: top, bottom left, bottom right.
fn dots<T>(pen: &Pen, colors: [Rgb565; 3], target: &mut T) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    for ((x, y), color) in [(0, -5), (-6, 5), (6, 5)].into_iter().zip(colors) {
        pen.disc(x, y, 9, color).draw(target)?;
    }
    Ok(())
}

fn bell<T>(pen: &Pen, target: &mut T) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    let ink = pen.colors.ink;
    pen.disc(0, -4, 17, ink).draw(target)?;
    Rectangle::new(pen.at(-8, -4), Size::new(pen.len(17), pen.len(9)))
        .into_styled(pen.ink())
        .draw(target)?;
    pen.rounded(-12, 4, 25, 4, 2).draw(target)?;
    pen.disc(0, 11, 5, ink).draw(target)
}
