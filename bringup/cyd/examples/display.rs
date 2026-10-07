#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use cyd_bringup::link::Link;
use cyd_bringup::panel::Panel;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::dma::DmaTxBuf;
use esp_hal::dma_tx_buffer;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use wade_core::render::{Band, ROW_BYTES};
use wade_core::{App, Settings, View};

esp_bootloader_esp_idf::esp_app_desc!();

/// Landscape, BGR: confirmed on this board in step 2.
const MADCTL: u8 = 0x28;
const ROWS: usize = 40;
const BAND_BYTES: usize = ROW_BYTES * ROWS;
const PHASE: Duration = Duration::from_secs(6);

/// Draw the strip at `top`: Wade's view, a 1-pixel white border, and `phase`
/// small squares in the top-left corner. Returns the bytes to send.
fn draw(buf: &mut [u8], top: u32, view: &View, phase: u32) -> usize {
    let mut band = Band::new(buf, top);
    band.draw(view);
    let white = PrimitiveStyle::with_stroke(Rgb565::WHITE, 1);
    let Ok(()) = Rectangle::new(Point::zero(), Size::new(320, 240))
        .into_styled(white)
        .draw(&mut band);
    for i in 0..phase {
        let x = 6 + 14 * i32::try_from(i).unwrap();
        let Ok(()) = Rectangle::new(Point::new(x, 6), Size::new(10, 10))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
            .draw(&mut band);
    }
    band.bytes().len()
}

/// Send frames through two DMA buffers, drawing one strip while the last is sent.
fn dma_frame(link: &mut Link, bufs: &mut [Option<DmaTxBuf>; 2], view: &View, phase: u32, window: bool) {
    if window {
        link.begin_frame();
    } else {
        link.begin_pixels();
    }
    let mut tops = Band::tops(ROWS);
    let mut ready = bufs[0].take().unwrap();
    let mut len = draw(ready.as_mut_slice(), tops.next().unwrap(), view, phase);
    let mut spare = bufs[1].take().unwrap();
    loop {
        let transfer = link.start(ready, len);
        let next = tops.next().map(|top| draw(spare.as_mut_slice(), top, view, phase));
        let sent = link.finish(transfer);
        match next {
            Some(n) => {
                ready = spare;
                spare = sent;
                len = n;
            }
            None => {
                bufs[0] = Some(sent);
                bufs[1] = Some(spare);
                break;
            }
        }
    }
    link.end();
}

/// Draw a strip, send it, wait for it, then draw the next: no CPU work during DMA.
fn dma_frame_sequential(link: &mut Link, buf: &mut Option<DmaTxBuf>, view: &View, phase: u32) {
    link.begin_frame();
    for top in Band::tops(ROWS) {
        let mut b = buf.take().unwrap();
        let len = draw(b.as_mut_slice(), top, view, phase);
        let transfer = link.start(b, len);
        *buf = Some(link.finish(transfer));
    }
    link.end();
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let slow = Config::default()
        .with_frequency(Rate::from_mhz(4))
        .with_mode(Mode::_0);
    let spi = Spi::new(peripherals.SPI2, slow)
        .unwrap()
        .with_sck(peripherals.GPIO14)
        .with_mosi(peripherals.GPIO13)
        .with_miso(peripherals.GPIO12);
    let mut panel = Panel {
        spi,
        dc: Output::new(peripherals.GPIO2, Level::High, OutputConfig::default()),
        cs: Output::new(peripherals.GPIO15, Level::High, OutputConfig::default()),
    };
    let _backlight = Output::new(peripherals.GPIO21, Level::High, OutputConfig::default());
    panel.init(MADCTL, false).await;

    let mut bufs = [
        Some(dma_tx_buffer!(BAND_BYTES).unwrap()),
        Some(dma_tx_buffer!(BAND_BYTES).unwrap()),
    ];
    // A fixed view, redrawn unchanged: any movement on screen comes from the transport.
    let view = App::new(wade_core::Instant::from_millis(0), 42, Settings::DEFAULT).view();

    for (phase, mhz) in [] as [(u32, u32); 0] {
        panel.spi.apply_config(&slow.with_frequency(Rate::from_mhz(mhz))).unwrap();
        info!("phase {phase}: plain SPI at {mhz} MHz");
        let buf = bufs[0].as_mut().unwrap().as_mut_slice();
        let end = Instant::now() + PHASE;
        while Instant::now() < end {
            panel.frame(buf, |b, top| draw(b, top, &view, phase), ROWS);
        }
    }

    let Panel { spi, dc, cs } = panel;
    let mut link = Link::new(spi.with_dma(peripherals.DMA_SPI2), dma_tx_buffer!(64).unwrap(), dc, cs);
    loop {
        for (phase, overlap) in [(4, true), (6, false)] {
            link.apply_config(&slow.with_frequency(Rate::from_mhz(80)));
            info!("phase {phase}: DMA at 80 MHz, overlapped: {overlap}");
            let end = Instant::now() + PHASE;
            while Instant::now() < end {
                if overlap {
                    dma_frame(&mut link, &mut bufs, &view, phase, true);
                } else {
                    dma_frame_sequential(&mut link, &mut bufs[0], &view, phase);
                }
            }
        }
    }
}
