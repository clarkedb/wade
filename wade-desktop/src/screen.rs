//! The simulated display, drawn whole or, with `--band-rows`, one strip at a
//! time as a low-memory board would (docs/ui.md#banded-rendering).

use embedded_graphics::pixelcolor::{Rgb565, raw::RawU16};
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::{SimulatorDisplay, Window};
use wade_core::render::{self, Band, ROW_BYTES};
use wade_core::{View, layout};

pub struct Screen {
    display: SimulatorDisplay<Rgb565>,
    /// The strip buffer, when drawing in bands.
    band: Option<Vec<u8>>,
}

impl Screen {
    pub fn new(band_rows: Option<usize>) -> Self {
        Screen {
            display: SimulatorDisplay::new(layout::SCREEN_SIZE),
            band: band_rows.map(|rows| vec![0; ROW_BYTES * rows]),
        }
    }

    /// Draw `view` and show it in `window`.
    pub fn show(&mut self, view: &View, window: &mut Window) {
        match &mut self.band {
            None => {
                let Ok(()) = render::draw(view, &mut self.display);
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
                    let Ok(()) = self.display.fill_contiguous(&band.area(), colors);
                }
            }
        }
        window.update(&self.display);
    }
}
