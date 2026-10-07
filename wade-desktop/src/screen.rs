//! The simulated display, drawn whole or, with `--band-rows`, one strip at a
//! time as a low-memory board would (docs/ui.md#banded-rendering). A simulated
//! backlight dims it to the brightness setting (D32).

use std::convert::Infallible;

use embedded_graphics::pixelcolor::{Rgb565, raw::RawU16};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics_simulator::{SimulatorDisplay, Window};
use wade_core::render::{self, Band, ROW_BYTES};
use wade_core::settings::Brightness;
use wade_core::{View, layout};

pub struct Screen {
    display: SimulatorDisplay<Rgb565>,
    /// The strip buffer, when drawing in bands.
    band: Option<Vec<u8>>,
    brightness: Brightness,
}

impl Screen {
    pub fn new(band_rows: Option<usize>, brightness: Brightness) -> Self {
        Screen {
            display: SimulatorDisplay::new(layout::SCREEN_SIZE),
            band: band_rows.map(|rows| vec![0; ROW_BYTES * rows]),
            brightness,
        }
    }

    /// Takes effect from the next `show`.
    pub fn set_brightness(&mut self, brightness: Brightness) {
        self.brightness = brightness;
    }

    /// Draw `view` and show it in `window`.
    pub fn show(&mut self, view: &View, window: &mut Window) {
        let mut display = Backlight::new(&mut self.display, self.brightness);
        match &mut self.band {
            None => {
                let Ok(()) = render::draw(view, &mut display);
            }
            Some(buf) => {
                let rows = buf.len() / ROW_BYTES;
                for top in Band::tops(rows) {
                    let mut band = Band::new(buf, top);
                    band.draw(view);
                    // Decode the panel's bytes, as the display would.
                    let colors = band
                        .bytes()
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|&pixel| Rgb565::from(RawU16::new(u16::from_be_bytes(pixel))));
                    let Ok(()) = display.fill_contiguous(&band.area(), colors);
                }
            }
        }
        window.update(&self.display);
    }
}

/// Dims every pixel drawn through it, as a backlight at `brightness` would.
struct Backlight<'a> {
    display: &'a mut SimulatorDisplay<Rgb565>,
    /// What each channel value is multiplied by.
    scale: f32,
}

impl<'a> Backlight<'a> {
    fn new(display: &'a mut SimulatorDisplay<Rgb565>, brightness: Brightness) -> Self {
        // A backlight scales emitted light. Pixel values are gamma-encoded, so
        // the same change in light is a smaller change in value.
        let light = f32::from(brightness.percent()) / 100.0;
        Backlight {
            display,
            scale: light.powf(1.0 / 2.2),
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a scale of at most 1 keeps each channel within its range"
    )]
    fn dim(&self, color: Rgb565) -> Rgb565 {
        let channel = |c: u8| (f32::from(c) * self.scale).round() as u8;
        Rgb565::new(channel(color.r()), channel(color.g()), channel(color.b()))
    }
}

impl OriginDimensions for Backlight<'_> {
    fn size(&self) -> Size {
        self.display.size()
    }
}

impl DrawTarget for Backlight<'_> {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Infallible>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        let dimmed: Vec<_> = pixels
            .into_iter()
            .map(|Pixel(p, c)| Pixel(p, self.dim(c)))
            .collect();
        self.display.draw_iter(dimmed)
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb565) -> Result<(), Infallible> {
        let color = self.dim(color);
        self.display.fill_solid(area, color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_brightness_leaves_colors_alone_and_lower_levels_dim_them() {
        let mut display = SimulatorDisplay::new(Size::new(1, 1));
        let full = Backlight::new(&mut display, Brightness::Full);
        assert_eq!(full.dim(Rgb565::WHITE), Rgb565::WHITE);
        let mut last = Rgb565::WHITE;
        for level in [
            Brightness::ThreeQuarters,
            Brightness::Half,
            Brightness::Quarter,
        ] {
            let dimmed = Backlight::new(&mut display, level).dim(Rgb565::WHITE);
            assert!(
                dimmed.g() < last.g(),
                "{level:?} is dimmer than the level above"
            );
            last = dimmed;
        }
        assert!(
            last.g() > Rgb565::WHITE.g() / 4,
            "a quarter of the light is more than a quarter of the value"
        );
    }
}
