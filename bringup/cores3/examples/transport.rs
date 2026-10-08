#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Step 5: transport stability and flush timing over DMA, for full frames and for the blink and
//! expression-change windows, at 40 and 80 MHz.

use cores3_bringup::link::Link;
use cores3_bringup::panel::Panel;
use cores3_bringup::power;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::dma::DmaTxBuf;
use esp_hal::dma_tx_buffer;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{self, I2c};
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use wade_core::render::{Band, ROW_BYTES};
use wade_core::{App, Settings, View};

esp_bootloader_esp_idf::esp_app_desc!();

const MADCTL: u8 = 0x08;
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

/// Send a frame through two DMA buffers, drawing one strip while the last is sent.
fn overlapped(link: &mut Link, bufs: &mut [Option<DmaTxBuf>; 2], view: &View, phase: u32) {
    link.begin_frame();
    let mut tops = Band::tops(ROWS);
    let mut ready = bufs[0].take().unwrap();
    let mut len = draw(ready.as_mut_slice(), tops.next().unwrap(), view, phase);
    let mut spare = bufs[1].take().unwrap();
    loop {
        let transfer = link.start(ready, len);
        let next = tops
            .next()
            .map(|top| draw(spare.as_mut_slice(), top, view, phase));
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

/// Draw a strip, send it, wait for it, then draw the next.
fn sequential(link: &mut Link, buf: &mut Option<DmaTxBuf>, view: &View, phase: u32) {
    link.begin_frame();
    for top in Band::tops(ROWS) {
        let mut b = buf.take().unwrap();
        let len = draw(b.as_mut_slice(), top, view, phase);
        let transfer = link.start(b, len);
        *buf = Some(link.finish(transfer));
    }
    link.end();
}

/// Send the bytes already in `buf` as a full frame: the bus alone.
fn bus_only(link: &mut Link, buf: &mut Option<DmaTxBuf>) {
    link.begin_frame();
    for _ in Band::tops(ROWS) {
        let transfer = link.start(buf.take().unwrap(), BAND_BYTES);
        *buf = Some(link.finish(transfer));
    }
    link.end();
}

/// Send a `w`×`h` window at the screen's center from `buf`, in chunks of whole rows.
fn window(link: &mut Link, buf: &mut Option<DmaTxBuf>, w: u16, h: u16) {
    let (x, y) = ((320 - w) / 2, (240 - h) / 2);
    link.set_window(x, y, x + w - 1, y + h - 1);
    link.begin_pixels();
    let mut left = usize::from(w) * usize::from(h) * 2;
    let chunk = BAND_BYTES / (usize::from(w) * 2) * usize::from(w) * 2;
    while left > 0 {
        let n = left.min(chunk);
        let transfer = link.start(buf.take().unwrap(), n);
        *buf = Some(link.finish(transfer));
        left -= n;
    }
    link.end();
}

fn time(label: &str, mut f: impl FnMut()) {
    let n = 30;
    let start = Instant::now();
    for _ in 0..n {
        f();
    }
    info!("  {label}: {} us", start.elapsed().as_micros() / n);
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);
    Timer::after(Duration::from_secs(2)).await;

    let mut i2c = I2c::new(
        peripherals.I2C0,
        master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO12)
    .with_scl(peripherals.GPIO11);
    power::init(&mut i2c).await.unwrap();

    let config = Config::default()
        .with_frequency(Rate::from_mhz(40))
        .with_mode(Mode::_0);
    let spi = Spi::new(peripherals.SPI2, config)
        .unwrap()
        .with_sck(peripherals.GPIO36)
        .with_mosi(peripherals.GPIO37);
    let mut panel = Panel {
        spi,
        dc: Output::new(peripherals.GPIO35, Level::High, OutputConfig::default()),
        cs: Output::new(peripherals.GPIO3, Level::High, OutputConfig::default()),
    };
    panel.init(MADCTL, true).await;
    let Panel { spi, dc, cs } = panel;
    let mut link = Link::new(
        spi.with_dma(peripherals.DMA_CH0),
        dma_tx_buffer!(64).unwrap(),
        dc,
        cs,
    );
    let mut bufs = [
        Some(dma_tx_buffer!(BAND_BYTES).unwrap()),
        Some(dma_tx_buffer!(BAND_BYTES).unwrap()),
    ];
    // A fixed view, redrawn unchanged: any movement on screen comes from the transport.
    let view = App::new(wade_core::Instant::from_millis(0), 42, Settings::DEFAULT).view();

    for mhz in [40, 80] {
        link.apply_config(&config.with_frequency(Rate::from_mhz(mhz)));
        info!("timing at {mhz} MHz");
        let [a, b] = &mut bufs;
        time("drawing Wade, all bands", || {
            for top in Band::tops(ROWS) {
                draw(a.as_mut().unwrap().as_mut_slice(), top, &view, 0);
            }
        });
        time("full frame, bus only", || bus_only(&mut link, b));
        time("full frame, draw then send", || {
            sequential(&mut link, a, &view, 0);
        });
        time("full frame, draw while sending", || {
            overlapped(&mut link, &mut bufs, &view, 0);
        });
        let [_, b] = &mut bufs;
        time("blink window 210x90", || window(&mut link, b, 210, 90));
        time("expression window 260x150", || {
            window(&mut link, b, 260, 150)
        });
    }

    loop {
        for (phase, mhz, overlap) in [(1, 40, false), (2, 40, true), (3, 80, false), (4, 80, true)]
        {
            link.apply_config(&config.with_frequency(Rate::from_mhz(mhz)));
            info!("phase {phase}: DMA at {mhz} MHz, overlapped: {overlap}");
            let end = Instant::now() + PHASE;
            while Instant::now() < end {
                if overlap {
                    overlapped(&mut link, &mut bufs, &view, phase);
                } else {
                    sequential(&mut link, &mut bufs[0], &view, phase);
                }
            }
        }
    }
}
