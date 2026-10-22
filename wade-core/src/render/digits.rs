//! Rounded seven-segment digits drawn from primitives, so they need no font
//! (docs/ui.md#text).

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle},
};

use crate::timer::Digits;

/// Segments, one bit each: top, upper right, lower right, bottom, lower left,
/// upper left, and middle.
const A: u8 = 1;
const B: u8 = 1 << 1;
const C: u8 = 1 << 2;
const D: u8 = 1 << 3;
const E: u8 = 1 << 4;
const F: u8 = 1 << 5;
const G: u8 = 1 << 6;

/// The segments lit for 0–9.
const NUMERALS: [u8; 10] = [
    A | B | C | D | E | F,
    B | C,
    A | B | D | E | G,
    A | B | C | D | G,
    B | C | F | G,
    A | C | D | F | G,
    A | C | D | E | F | G,
    A | B | C,
    A | B | C | D | E | F | G,
    A | B | C | D | F | G,
];

/// The proportions of a line of digits.
#[derive(Clone, Copy)]
pub struct Scale {
    /// Size of one digit.
    width: i32,
    height: i32,
    /// Segment thickness; each segment is a pill with fully rounded ends.
    thickness: i32,
    /// Space between the ends of neighboring segments.
    gap: i32,
    /// Space between digits, and the width the colon takes between minutes and seconds.
    spacing: i32,
    colon: i32,
}

/// The Timer screen's digits.
pub const LARGE: Scale = Scale {
    width: 36,
    height: 64,
    thickness: 8,
    gap: 3,
    spacing: 10,
    colon: 24,
};

/// The time as MM:SS at `scale` in `color`, centered in `band`.
pub fn time<T>(
    digits: Digits,
    band: Rectangle,
    scale: Scale,
    color: Rgb565,
    target: &mut T,
) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    let Scale {
        width,
        height,
        thickness,
        spacing,
        colon,
        ..
    } = scale;
    let (minutes, seconds) = (digits.minutes(), digits.seconds());
    let total = 4 * width + 2 * spacing + colon;
    let top_left = band.top_left
        + Point::new(
            (band.size.width.cast_signed() - total) / 2,
            (band.size.height.cast_signed() - height) / 2,
        );
    let pair = width + spacing + width;
    let numerals = [minutes / 10, minutes % 10, seconds / 10, seconds % 10]
        .map(|n| NUMERALS[usize::from(n % 10)]);
    for (i, segments) in (0..).zip(numerals) {
        let shift = if i >= 2 { colon - spacing } else { 0 };
        let x = i * (width + spacing) + shift;
        glyph(segments, top_left + Point::new(x, 0), scale, color, target)?;
    }
    // From the top-left corner, so each dot sits exactly centered in the colon's space.
    let x = top_left.x + pair + (colon - thickness) / 2;
    for y in [height / 3, height - height / 3] {
        Circle::new(
            Point::new(x, top_left.y + y - thickness / 2),
            thickness.cast_unsigned(),
        )
        .into_styled(PrimitiveStyle::with_fill(color))
        .draw(target)?;
    }
    Ok(())
}

/// One digit with `segments` lit, its top-left corner at `at`.
fn glyph<T>(
    segments: u8,
    at: Point,
    scale: Scale,
    color: Rgb565,
    target: &mut T,
) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    let Scale {
        width,
        height,
        thickness,
        gap,
        ..
    } = scale;
    // Each pill ends `gap` short of the corner where its neighbors meet.
    let end = thickness / 2 + gap;
    let across = width - 2 * end;
    let down = height / 2 - thickness / 2 - 2 * gap;
    let horizontal = |y| Rectangle::new(at + Point::new(end, y), size(across, thickness));
    let vertical = |x, y| Rectangle::new(at + Point::new(x, y), size(thickness, down));
    let right = width - thickness;
    let upper = end;
    let lower = height / 2 + gap;
    let pills = [
        (A, horizontal(0)),
        (B, vertical(right, upper)),
        (C, vertical(right, lower)),
        (D, horizontal(height - thickness)),
        (E, vertical(0, lower)),
        (F, vertical(0, upper)),
        (G, horizontal((height - thickness) / 2)),
    ];
    let radius = size(thickness / 2, thickness / 2);
    for (segment, area) in pills {
        if segments & segment != 0 {
            RoundedRectangle::with_equal_corners(area, radius)
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(target)?;
        }
    }
    Ok(())
}

const fn size(w: i32, h: i32) -> Size {
    Size::new(w.cast_unsigned(), h.cast_unsigned())
}
