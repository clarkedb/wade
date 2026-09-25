//! Rounded seven-segment digits drawn from primitives, so they need no font
//! (docs/ui.md#text).

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle},
};

use crate::layout;
use crate::render::palette;
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

/// Size of one digit.
const WIDTH: i32 = 36;
const HEIGHT: i32 = 64;
/// Segment thickness; each segment is a pill with fully rounded ends.
const THICKNESS: i32 = 8;
/// Space between the ends of neighboring segments.
const GAP: i32 = 3;
/// Space between digits, and the width the colon takes between minutes and seconds.
const SPACING: i32 = 10;
const COLON: i32 = 24;

/// The time as MM:SS, centered in the digits band.
pub fn time<T>(digits: Digits, target: &mut T) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    let (m, s) = (digits.minutes(), digits.seconds());
    let width = 4 * WIDTH + 2 * SPACING + COLON;
    let top_left = origin(width);
    let pair = WIDTH + SPACING + WIDTH;
    let numerals = [m / 10, m % 10, s / 10, s % 10].map(|n| NUMERALS[usize::from(n % 10)]);
    for (i, segments) in (0..).zip(numerals) {
        let colon = if i >= 2 { COLON - SPACING } else { 0 };
        let x = i * (WIDTH + SPACING) + colon;
        glyph(segments, top_left + Point::new(x, 0), target)?;
    }
    // From the top-left corner, so each dot sits exactly centered in the colon's space.
    let x = top_left.x + pair + (COLON - THICKNESS) / 2;
    for y in [HEIGHT / 3, HEIGHT - HEIGHT / 3] {
        Circle::new(
            Point::new(x, top_left.y + y - THICKNESS / 2),
            THICKNESS.cast_unsigned(),
        )
        .into_styled(PrimitiveStyle::with_fill(palette::EYE))
        .draw(target)?;
    }
    Ok(())
}

/// Where a line of digits `width` wide starts, centered in the digits band.
fn origin(width: i32) -> Point {
    let band = layout::TIMER_DIGITS;
    let (w, h) = (
        band.size.width.cast_signed(),
        band.size.height.cast_signed(),
    );
    band.top_left + Point::new((w - width) / 2, (h - HEIGHT) / 2)
}

/// One digit with `segments` lit, its top-left corner at `at`.
fn glyph<T>(segments: u8, at: Point, target: &mut T) -> Result<(), T::Error>
where
    T: DrawTarget<Color = Rgb565>,
{
    // Each pill ends GAP short of the corner where its neighbors meet.
    let end = THICKNESS / 2 + GAP;
    let across = WIDTH - 2 * end;
    let down = HEIGHT / 2 - THICKNESS / 2 - 2 * GAP;
    let horizontal = |y| Rectangle::new(at + Point::new(end, y), size(across, THICKNESS));
    let vertical = |x, y| Rectangle::new(at + Point::new(x, y), size(THICKNESS, down));
    let right = WIDTH - THICKNESS;
    let upper = end;
    let lower = HEIGHT / 2 + GAP;
    let pills = [
        (A, horizontal(0)),
        (B, vertical(right, upper)),
        (C, vertical(right, lower)),
        (D, horizontal(HEIGHT - THICKNESS)),
        (E, vertical(0, lower)),
        (F, vertical(0, upper)),
        (G, horizontal((HEIGHT - THICKNESS) / 2)),
    ];
    let radius = size(THICKNESS / 2, THICKNESS / 2);
    for (segment, area) in pills {
        if segments & segment != 0 {
            RoundedRectangle::with_equal_corners(area, radius)
                .into_styled(PrimitiveStyle::with_fill(palette::EYE))
                .draw(target)?;
        }
    }
    Ok(())
}

const fn size(w: i32, h: i32) -> Size {
    Size::new(w.cast_unsigned(), h.cast_unsigned())
}
