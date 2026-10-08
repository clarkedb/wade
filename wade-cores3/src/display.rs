//! The CoreS3's ILI9342C panel over SPI with DMA (docs/hardware-notes.md#cores3-lite).
//! Its reset is pulsed by `power::init`; MISO shares the DC pin, so it is never read.
//!
//! Frames are drawn in bands (docs/ui.md#banded-rendering): one band is drawn
//! while the last is sent, from two DMA buffers that the bands borrow in turn.

use embassy_time::{Duration, Timer};
use esp_hal::Async;
use esp_hal::dma::DmaTxBuf;
use esp_hal::gpio::Output;
use esp_hal::spi::master::{SpiDma, SpiDmaTransfer};
use wade_core::View;
use wade_core::render::{Band, ROW_BYTES};

/// Rows per band. Two bands of this height are the display's whole RAM cost.
pub const BAND_ROWS: usize = 40;
pub const BAND_BYTES: usize = ROW_BYTES * BAND_ROWS;
/// Room for the longest command parameter list.
pub const COMMAND_BYTES: usize = 16;

/// The panel's native landscape, BGR.
const MADCTL: u8 = 0x08;

const SWRESET: u8 = 0x01;
const SLPOUT: u8 = 0x11;
const INVON: u8 = 0x21;
const DISPON: u8 = 0x29;
const CASET: u8 = 0x2A;
const PASET: u8 = 0x2B;
const RAMWR: u8 = 0x2C;
const MADCTL_CMD: u8 = 0x36;
const COLMOD: u8 = 0x3A;
/// 16 bits per pixel.
const RGB565: u8 = 0x55;

type Transfer<'d> = SpiDmaTransfer<'d, Async, DmaTxBuf>;

pub struct Display<'d> {
    /// Held here between transfers; a transfer owns it while it runs.
    spi: Option<SpiDma<'d, Async>>,
    command: Option<DmaTxBuf>,
    bands: [Option<DmaTxBuf>; 2],
    dc: Output<'d>,
    cs: Output<'d>,
}

impl<'d> Display<'d> {
    /// Initialize the panel and leave it on, showing whatever its memory holds.
    pub async fn new(
        spi: SpiDma<'d, Async>,
        command: DmaTxBuf,
        bands: [DmaTxBuf; 2],
        dc: Output<'d>,
        cs: Output<'d>,
    ) -> Self {
        let [a, b] = bands;
        let mut display = Display {
            spi: Some(spi),
            command: Some(command),
            bands: [Some(a), Some(b)],
            dc,
            cs,
        };
        display.command(SWRESET, &[]).await;
        Timer::after(Duration::from_millis(150)).await;
        display.command(SLPOUT, &[]).await;
        Timer::after(Duration::from_millis(120)).await;
        display.command(COLMOD, &[RGB565]).await;
        display.command(MADCTL_CMD, &[MADCTL]).await;
        display.command(INVON, &[]).await;
        display.command(DISPON, &[]).await;
        display
    }

    /// Draw `view` and send it to the whole screen.
    pub async fn show(&mut self, view: &View) {
        self.command(CASET, &[0, 0, 0x01, 0x3F]).await;
        self.command(PASET, &[0, 0, 0, 0xEF]).await;
        self.cs.set_low();
        self.dc.set_low();
        self.write_command_bytes(&[RAMWR]).await;
        self.dc.set_high();

        let mut tops = Band::tops(BAND_ROWS);
        let mut ready = self.bands[0]
            .take()
            .expect("bands are returned after each frame");
        let mut spare = self.bands[1]
            .take()
            .expect("bands are returned after each frame");
        let first = tops.next().expect("the screen has at least one band");
        let mut len = draw(&mut ready, view, first);
        loop {
            let mut transfer = self.start(ready, len);
            let next = tops.next().map(|top| draw(&mut spare, view, top));
            transfer.wait_for_done().await;
            let sent = self.finish(transfer);
            let Some(n) = next else {
                self.bands = [Some(sent), Some(spare)];
                break;
            };
            ready = spare;
            spare = sent;
            len = n;
        }
        self.cs.set_high();
    }

    async fn command(&mut self, cmd: u8, params: &[u8]) {
        self.cs.set_low();
        self.dc.set_low();
        self.write_command_bytes(&[cmd]).await;
        self.dc.set_high();
        if !params.is_empty() {
            self.write_command_bytes(params).await;
        }
        self.cs.set_high();
    }

    async fn write_command_bytes(&mut self, bytes: &[u8]) {
        let mut buf = self
            .command
            .take()
            .expect("the command buffer is returned after each write");
        buf.as_mut_slice()[..bytes.len()].copy_from_slice(bytes);
        let mut transfer = self.start(buf, bytes.len());
        transfer.wait_for_done().await;
        self.command = Some(self.finish(transfer));
    }

    fn start(&mut self, mut buf: DmaTxBuf, len: usize) -> Transfer<'d> {
        buf.set_length(len);
        let spi = self
            .spi
            .take()
            .expect("the SPI bus is returned after each transfer");
        match spi.write_buffer(len, buf) {
            Ok(transfer) => transfer,
            Err((e, _, _)) => panic!("display DMA write failed: {e:?}"),
        }
    }

    fn finish(&mut self, transfer: Transfer<'d>) -> DmaTxBuf {
        let (spi, buf) = transfer.wait();
        self.spi = Some(spi);
        buf
    }
}

/// Draw `view`'s band at `top` into `buf`, returning the bytes to send.
fn draw(buf: &mut DmaTxBuf, view: &View, top: u32) -> usize {
    let mut band = Band::new(buf.as_mut_slice(), top);
    band.draw(view);
    band.bytes().len()
}
