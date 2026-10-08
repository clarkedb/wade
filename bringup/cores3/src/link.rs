//! The panel over SPI with DMA. Commands go through a small DMA buffer; pixel
//! strips go straight from the band buffers with no copy.

use esp_hal::Blocking;
use esp_hal::dma::DmaTxBuf;
use esp_hal::gpio::Output;
use esp_hal::spi::master::{Config, SpiDma, SpiDmaTransfer};

pub type Transfer<'d> = SpiDmaTransfer<'d, Blocking, DmaTxBuf>;

pub struct Link<'d> {
    spi: Option<SpiDma<'d, Blocking>>,
    cmd: Option<DmaTxBuf>,
    dc: Output<'d>,
    cs: Output<'d>,
}

impl<'d> Link<'d> {
    pub fn new(spi: SpiDma<'d, Blocking>, cmd: DmaTxBuf, dc: Output<'d>, cs: Output<'d>) -> Self {
        Link {
            spi: Some(spi),
            cmd: Some(cmd),
            dc,
            cs,
        }
    }

    pub fn apply_config(&mut self, config: &Config) {
        self.spi.as_mut().unwrap().apply_config(config).unwrap();
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut buf = self.cmd.take().unwrap();
        buf.as_mut_slice()[..bytes.len()].copy_from_slice(bytes);
        buf.set_length(bytes.len());
        let transfer = self.start(buf, bytes.len());
        self.cmd = Some(self.finish(transfer));
    }

    pub fn command(&mut self, cmd: u8, params: &[u8]) {
        self.cs.set_low();
        self.dc.set_low();
        self.write(&[cmd]);
        self.dc.set_high();
        if !params.is_empty() {
            self.write(params);
        }
        self.cs.set_high();
    }

    /// Start a memory write to the whole screen. The caller streams strips top
    /// to bottom, then calls `end`.
    pub fn begin_frame(&mut self) {
        self.set_window(0, 0, 319, 239);
        self.begin_pixels();
    }

    /// Set the write window from (x0, y0) to (x1, y1) inclusive.
    pub fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) {
        let [a, b] = x0.to_be_bytes();
        let [c, d] = x1.to_be_bytes();
        self.command(0x2A, &[a, b, c, d]);
        let [a, b] = y0.to_be_bytes();
        let [c, d] = y1.to_be_bytes();
        self.command(0x2B, &[a, b, c, d]);
    }

    /// RAMWR without touching the window: the panel restarts at its top-left.
    pub fn begin_pixels(&mut self) {
        self.cs.set_low();
        self.dc.set_low();
        self.write(&[0x2C]);
        self.dc.set_high();
    }

    pub fn end(&mut self) {
        self.cs.set_high();
    }

    /// Start sending the first `len` bytes of `buf`.
    pub fn start(&mut self, mut buf: DmaTxBuf, len: usize) -> Transfer<'d> {
        buf.set_length(len);
        match self.spi.take().unwrap().write_buffer(len, buf) {
            Ok(transfer) => transfer,
            Err((e, _, _)) => panic!("dma write: {e:?}"),
        }
    }

    /// Wait for `transfer` and take its buffer back.
    pub fn finish(&mut self, transfer: Transfer<'d>) -> DmaTxBuf {
        let (spi, buf) = transfer.wait();
        self.spi = Some(spi);
        buf
    }
}
