//! A bare ILI9341/ST7789 driver over blocking SPI: just enough to identify the
//! controller, initialize it, and push pixels.

use embassy_time::{Duration, Timer};
use esp_hal::Blocking;
use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;

pub const WIDTH: u16 = 320;
pub const HEIGHT: u16 = 240;

pub struct Panel<'d> {
    pub spi: Spi<'d, Blocking>,
    pub dc: Output<'d>,
    pub cs: Output<'d>,
}

impl Panel<'_> {
    /// Send `cmd` followed by its parameters.
    pub fn command(&mut self, cmd: u8, params: &[u8]) {
        self.cs.set_low();
        self.dc.set_low();
        self.spi.write(&[cmd]).unwrap();
        self.dc.set_high();
        if !params.is_empty() {
            self.spi.write(params).unwrap();
        }
        self.cs.set_high();
    }

    /// Send `cmd`, then clock in `out.len()` bytes. Reads must run at a low SPI clock.
    pub fn read(&mut self, cmd: u8, out: &mut [u8]) {
        self.cs.set_low();
        self.dc.set_low();
        self.spi.write(&[cmd]).unwrap();
        self.dc.set_high();
        self.spi.read(out).unwrap();
        self.cs.set_high();
    }

    pub async fn init(&mut self, madctl: u8, invert: bool) {
        self.command(0x01, &[]); // software reset
        Timer::after(Duration::from_millis(150)).await;
        self.command(0x11, &[]); // sleep out
        Timer::after(Duration::from_millis(120)).await;
        self.command(0x3A, &[0x55]); // 16 bits per pixel
        self.command(0x36, &[madctl]);
        self.command(if invert { 0x21 } else { 0x20 }, &[]);
        self.command(0x29, &[]); // display on
    }

    /// Start a memory write to the window from (x0, y0) to (x1, y1) inclusive.
    /// Leaves CS low and DC high: the caller streams pixels, then calls `end`.
    pub fn begin(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) {
        let [a, b] = x0.to_be_bytes();
        let [c, d] = x1.to_be_bytes();
        self.command(0x2A, &[a, b, c, d]);
        let [a, b] = y0.to_be_bytes();
        let [c, d] = y1.to_be_bytes();
        self.command(0x2B, &[a, b, c, d]);
        self.cs.set_low();
        self.dc.set_low();
        self.spi.write(&[0x2C]).unwrap();
        self.dc.set_high();
    }

    /// Write a whole frame from strips drawn by `draw` into `buf`.
    pub fn frame(&mut self, buf: &mut [u8], draw: impl Fn(&mut [u8], u32) -> usize, rows: usize) {
        self.begin(0, 0, WIDTH - 1, HEIGHT - 1);
        for top in wade_core::render::Band::tops(rows) {
            let len = draw(buf, top);
            self.spi.write(&buf[..len]).unwrap();
        }
        self.end();
    }

    pub fn end(&mut self) {
        self.cs.set_high();
    }

    /// Fill a rectangle with one Rgb565 color.
    pub fn fill(&mut self, x: u16, y: u16, w: u16, h: u16, color: u16) {
        let mut row = [0u8; WIDTH as usize * 2];
        let [hi, lo] = color.to_be_bytes();
        for px in row[..w as usize * 2].chunks_exact_mut(2) {
            px[0] = hi;
            px[1] = lo;
        }
        self.begin(x, y, x + w - 1, y + h - 1);
        for _ in 0..h {
            self.spi.write(&row[..w as usize * 2]).unwrap();
        }
        self.end();
    }
}

pub const RED: u16 = 0xF800;
pub const GREEN: u16 = 0x07E0;
pub const BLUE: u16 = 0x001F;
pub const WHITE: u16 = 0xFFFF;
pub const BLACK: u16 = 0x0000;
