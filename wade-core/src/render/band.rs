//! Drawing a frame one horizontal strip at a time, for platforms without the
//! RAM for a full framebuffer (see docs/ui.md#banded-rendering).

use core::convert::Infallible;
use core::fmt;

use embedded_graphics::{pixelcolor::Rgb565, prelude::*, primitives::Rectangle};

use crate::layout::{SCREEN, SCREEN_SIZE};
use crate::view::View;

const WIDTH: usize = SCREEN_SIZE.width as usize;

/// Bytes in one full-width row of Rgb565 pixels. A band's buffer holds a whole
/// number of rows; any remainder is unused.
pub const ROW_BYTES: usize = WIDTH * 2;

/// A strip of the screen, full width, drawn into a borrowed buffer as big-endian
/// Rgb565, the byte order SPI panels expect, so the buffer can be sent as is.
///
/// As a `DrawTarget` it covers the whole screen and keeps only the pixels inside
/// its area, so anything drawn for a full frame can be drawn into it.
pub struct Band<'a> {
    buf: &'a mut [u8],
    area: Rectangle,
}

impl<'a> Band<'a> {
    /// The strip from row `top` down, as many rows as `buf` holds, or fewer at
    /// the bottom of the screen. The buffer keeps whatever it held until drawn over.
    #[must_use]
    pub fn new(buf: &'a mut [u8], top: u32) -> Self {
        let top = top.min(SCREEN_SIZE.height);
        let rows = u32::try_from(buf.len() / ROW_BYTES).unwrap_or(u32::MAX);
        let area = Rectangle::new(
            Point::new(0, top.cast_signed()),
            Size::new(SCREEN_SIZE.width, rows.min(SCREEN_SIZE.height - top)),
        );
        Band { buf, area }
    }

    /// The top row of each strip of `rows` rows that covers the screen, top to
    /// bottom. None if `rows` is zero.
    pub fn tops(rows: usize) -> impl Iterator<Item = u32> {
        let end = if rows == 0 { 0 } else { SCREEN_SIZE.height };
        (0..end).step_by(rows.max(1))
    }

    /// Draw the band's share of `view`'s frame. `render::draw` paints every pixel,
    /// so nothing from an earlier frame or strip survives.
    pub fn draw(&mut self, view: &View) {
        let Ok(()) = super::draw(view, self);
    }

    /// The part of the screen the band holds.
    #[must_use]
    pub const fn area(&self) -> Rectangle {
        self.area
    }

    /// The band's pixels, big-endian Rgb565, row by row across `area`.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.buf[..self.area.size.height as usize * ROW_BYTES]
    }

    /// The byte offset of `p`, which must lie inside `area`.
    fn offset(&self, p: Point) -> usize {
        let p = p - self.area.top_left;
        (p.y.unsigned_abs() as usize * WIDTH + p.x.unsigned_abs() as usize) * 2
    }
}

impl fmt::Debug for Band<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Band")
            .field("area", &self.area)
            .finish_non_exhaustive()
    }
}

impl OriginDimensions for Band<'_> {
    fn size(&self) -> Size {
        SCREEN_SIZE
    }
}

impl DrawTarget for Band<'_> {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Infallible>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        for Pixel(p, color) in pixels {
            if self.area.contains(p) {
                let i = self.offset(p);
                self.buf[i..i + 2].copy_from_slice(&color.into_storage().to_be_bytes());
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb565) -> Result<(), Infallible> {
        let area = area.intersection(&self.area).intersection(&SCREEN);
        let Some(bottom_right) = area.bottom_right() else {
            return Ok(());
        };
        let bytes = color.into_storage().to_be_bytes();
        let width = area.size.width as usize * 2;
        for y in area.top_left.y..=bottom_right.y {
            let start = self.offset(Point::new(area.top_left.x, y));
            for pixel in self.buf[start..start + width].as_chunks_mut::<2>().0 {
                *pixel = bytes;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::raw::RawU16;
    use embedded_graphics::primitives::{PrimitiveStyle, StyledDrawable};
    use std::vec::Vec;

    fn pixel(band: &Band, x: usize, y: usize) -> Rgb565 {
        let i = (y * WIDTH + x) * 2;
        RawU16::new(u16::from_be_bytes([band.bytes()[i], band.bytes()[i + 1]])).into()
    }

    #[test]
    fn keeps_only_its_rows() {
        let mut buf = [0; ROW_BYTES * 10];
        let mut band = Band::new(&mut buf, 20);
        band.clear(Rgb565::BLACK).unwrap();
        // Straddles both edges of the band and runs off the left of the screen.
        Rectangle::new(Point::new(-5, 15), Size::new(10, 10))
            .draw_styled(&PrimitiveStyle::with_fill(Rgb565::RED), &mut band)
            .unwrap();
        Rectangle::new(Point::new(0, 100), Size::new(10, 10))
            .draw_styled(&PrimitiveStyle::with_fill(Rgb565::BLUE), &mut band)
            .unwrap();
        Pixel(Point::new(319, 29), Rgb565::GREEN)
            .draw(&mut band)
            .unwrap();
        for p in [
            Point::new(320, 29),
            Point::new(0, 30),
            Point::new(0, i32::MIN),
            Point::new(i32::MAX, i32::MAX),
        ] {
            Pixel(p, Rgb565::BLUE).draw(&mut band).unwrap();
        }

        assert_eq!(band.bytes().len(), ROW_BYTES * 10);
        for y in 0..10 {
            let red = if y < 5 { 5 } else { 0 };
            for x in 0..319 {
                let expected = if x < red { Rgb565::RED } else { Rgb565::BLACK };
                assert_eq!(pixel(&band, x, y), expected, "({x}, {y})");
            }
        }
        assert_eq!(pixel(&band, 319, 9), Rgb565::GREEN);
    }

    #[test]
    fn stores_big_endian() {
        let mut buf = [0; ROW_BYTES];
        let mut band = Band::new(&mut buf, 0);
        Pixel(Point::zero(), Rgb565::new(0b11111, 0, 0b00001))
            .draw(&mut band)
            .unwrap();
        assert_eq!(band.bytes()[..2], [0b1111_1000, 0b0000_0001]);
    }

    #[test]
    fn last_band_is_short() {
        assert_eq!(Band::tops(64).collect::<Vec<_>>(), [0, 64, 128, 192]);
        let mut buf = std::vec![0; ROW_BYTES * 64 + 5];
        let band = Band::new(&mut buf, 192);
        assert_eq!(
            band.area(),
            Rectangle::new(Point::new(0, 192), Size::new(320, 48))
        );
        assert_eq!(band.bytes().len(), ROW_BYTES * 48);
    }

    #[test]
    fn a_band_taller_than_the_screen_holds_all_of_it() {
        assert_eq!(Band::tops(300).collect::<Vec<_>>(), [0]);
        let mut buf = std::vec![0; ROW_BYTES * 300];
        assert_eq!(Band::new(&mut buf, 0).bytes().len(), ROW_BYTES * 240);
    }

    #[test]
    fn a_buffer_too_small_for_a_row_holds_nothing() {
        assert_eq!(Band::tops(0).count(), 0);
        let mut buf = [0; ROW_BYTES - 1];
        let mut band = Band::new(&mut buf, 0);
        band.clear(Rgb565::RED).unwrap();
        assert!(band.bytes().is_empty());
    }
}
