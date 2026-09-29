//! Drawing a frame one horizontal strip at a time, for platforms without the
//! RAM for a full framebuffer (see docs/ui.md#banded-rendering).

use core::convert::Infallible;

use embedded_graphics::{pixelcolor::Rgb565, prelude::*, primitives::Rectangle};

use crate::layout::SCREEN_SIZE;
use crate::view::View;

const WIDTH: usize = SCREEN_SIZE.width as usize;

/// A strip of the screen, full width and `H` rows tall, that holds its share of a frame.
///
/// As a `DrawTarget` it covers the whole screen and keeps only the pixels inside
/// its current area, so anything drawn for a full frame can be drawn into it.
#[derive(Debug)]
pub struct Band<const H: usize> {
    rows: [[Rgb565; WIDTH]; H],
    area: Rectangle,
}

impl<const H: usize> Band<H> {
    /// An empty band; `draw` fills it. `const` so a platform can place it in a `static`.
    #[must_use]
    pub const fn new() -> Self {
        const { assert!(H > 0, "a band needs at least one row") };
        Band {
            rows: [[Rgb565::BLACK; WIDTH]; H],
            area: Rectangle::zero(),
        }
    }

    /// The top row of each band that covers the screen, top to bottom.
    pub fn tops() -> impl Iterator<Item = u32> {
        (0..SCREEN_SIZE.height).step_by(H)
    }

    /// Draw the rows of `view`'s frame from `top` down: `H` rows, or fewer at the
    /// bottom of the screen. Afterwards `area` and `rows` describe them.
    pub fn draw(&mut self, view: &View, top: u32) {
        self.move_to(top);
        let Ok(()) = super::draw(view, self);
    }

    /// The part of the screen the band holds.
    #[must_use]
    pub const fn area(&self) -> Rectangle {
        self.area
    }

    /// The band's pixels, one full-width row per row of `area`.
    #[must_use]
    pub fn rows(&self) -> &[[Rgb565; WIDTH]] {
        &self.rows[..self.area.size.height as usize]
    }

    fn move_to(&mut self, top: u32) {
        let top = top.min(SCREEN_SIZE.height);
        let height = (SCREEN_SIZE.height - top).min(u32::try_from(H).unwrap_or(u32::MAX));
        self.area = Rectangle::new(
            Point::new(0, i32::try_from(top).unwrap_or(i32::MAX)),
            Size::new(SCREEN_SIZE.width, height),
        );
    }
}

impl<const H: usize> Default for Band<H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const H: usize> OriginDimensions for Band<H> {
    fn size(&self) -> Size {
        SCREEN_SIZE
    }
}

impl<const H: usize> DrawTarget for Band<H> {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Infallible>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        let top = self.area.top_left.y;
        let height = self.area.size.height as usize;
        for Pixel(p, color) in pixels {
            if let (Ok(x), Ok(y)) = (usize::try_from(p.x), usize::try_from(p.y - top))
                && x < WIDTH
                && y < height
            {
                self.rows[y][x] = color;
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb565) -> Result<(), Infallible> {
        let area = area.intersection(&self.area);
        let Some(bottom_right) = area.bottom_right() else {
            return Ok(());
        };
        let top = self.area.top_left.y;
        let (Ok(x0), Ok(x1), Ok(y0), Ok(y1)) = (
            usize::try_from(area.top_left.x),
            usize::try_from(bottom_right.x),
            usize::try_from(area.top_left.y - top),
            usize::try_from(bottom_right.y - top),
        ) else {
            return Ok(());
        };
        for row in &mut self.rows[y0..=y1] {
            row[x0..=x1].fill(color);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::primitives::{PrimitiveStyle, StyledDrawable};

    #[test]
    fn keeps_only_its_rows() {
        let mut band = Band::<10>::new();
        band.move_to(20);
        band.clear(Rgb565::BLACK).unwrap();
        // Straddles both edges of the band and runs off the left of the screen.
        Rectangle::new(Point::new(-5, 15), Size::new(10, 10))
            .draw_styled(&PrimitiveStyle::with_fill(Rgb565::RED), &mut band)
            .unwrap();
        Pixel(Point::new(319, 29), Rgb565::GREEN)
            .draw(&mut band)
            .unwrap();
        Pixel(Point::new(320, 29), Rgb565::BLUE)
            .draw(&mut band)
            .unwrap();
        Pixel(Point::new(0, 30), Rgb565::BLUE)
            .draw(&mut band)
            .unwrap();

        let rows = band.rows();
        assert_eq!(rows.len(), 10);
        for (y, row) in rows.iter().enumerate() {
            let red = if y < 5 { 5 } else { 0 };
            assert!(row[..red].iter().all(|&c| c == Rgb565::RED), "row {y}");
            assert!(row[red..319].iter().all(|&c| c == Rgb565::BLACK), "row {y}");
        }
        assert_eq!(rows[9][319], Rgb565::GREEN);
    }

    #[test]
    fn last_band_is_short() {
        let mut band = Band::<64>::new();
        assert_eq!(
            Band::<64>::tops().collect::<std::vec::Vec<_>>(),
            [0, 64, 128, 192]
        );
        band.move_to(192);
        assert_eq!(
            band.area(),
            Rectangle::new(Point::new(0, 192), Size::new(320, 48))
        );
        assert_eq!(band.rows().len(), 48);
    }

    #[test]
    fn a_band_taller_than_the_screen_holds_all_of_it() {
        let mut band = Band::<300>::new();
        assert_eq!(Band::<300>::tops().collect::<std::vec::Vec<_>>(), [0]);
        band.move_to(0);
        assert_eq!(band.rows().len(), 240);
    }
}
