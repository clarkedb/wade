#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Step 10: a full framebuffer in internal SRAM and in PSRAM, sent by DMA at 80 MHz. Times
//! drawing and flushing from each, then alternates them on screen with a moving square that
//! would show stale cache data.

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
use esp_hal::dma::aligned::{DmaAlignedMut, InternalMemory};
use esp_hal::dma_tx_buffer;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{self, I2c};
use esp_hal::psram::{Psram, PsramConfig, PsramMode};
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use static_cell::ConstStaticCell;
use wade_core::render::{Band, ROW_BYTES};
use wade_core::{App, Settings, View};

esp_bootloader_esp_idf::esp_app_desc!();

const MADCTL: u8 = 0x08;
const ROWS: usize = 48;
const CHUNK: usize = ROW_BYTES * ROWS;
const CHUNKS: usize = 5;
const FRAME: usize = CHUNK * CHUNKS;
const PHASE: Duration = Duration::from_secs(6);

static INTERNAL: ConstStaticCell<InternalMemory<[u8; FRAME]>> =
    ConstStaticCell::new(InternalMemory::new([0; FRAME]));

/// One DMA buffer per chunk of rows, each with its own descriptors.
macro_rules! frame {
    ($memory:expr) => {{
        let mut chunks = $memory.chunks_exact_mut(CHUNK);
        let mut next = || DmaAlignedMut::new(chunks.next().unwrap()).unwrap();
        [
            Some(DmaTxBuf::new(esp_hal::dma_descriptors_impl!(CHUNK, 2048), next()).unwrap()),
            Some(DmaTxBuf::new(esp_hal::dma_descriptors_impl!(CHUNK, 2048), next()).unwrap()),
            Some(DmaTxBuf::new(esp_hal::dma_descriptors_impl!(CHUNK, 2048), next()).unwrap()),
            Some(DmaTxBuf::new(esp_hal::dma_descriptors_impl!(CHUNK, 2048), next()).unwrap()),
            Some(DmaTxBuf::new(esp_hal::dma_descriptors_impl!(CHUNK, 2048), next()).unwrap()),
        ]
    }};
}

type Frame = [Option<DmaTxBuf>; CHUNKS];

/// Draw Wade, a border, `phase` squares in the top-left corner, and a square at `x` along the
/// bottom into every chunk.
fn draw(frame: &mut Frame, view: &View, phase: u32, x: i32) {
    for (i, chunk) in frame.iter_mut().enumerate() {
        let mut band = Band::new(chunk.as_mut().unwrap().as_mut_slice(), (i * ROWS) as u32);
        band.draw(view);
        let Ok(()) = Rectangle::new(Point::zero(), Size::new(320, 240))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 1))
            .draw(&mut band);
        for p in 0..phase {
            let Ok(()) = Rectangle::new(Point::new(6 + 14 * p as i32, 6), Size::new(10, 10))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
                .draw(&mut band);
        }
        let Ok(()) = Rectangle::new(Point::new(x, 220), Size::new(12, 12))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::CYAN))
            .draw(&mut band);
    }
}

fn flush(link: &mut Link, frame: &mut Frame) {
    link.begin_frame();
    for chunk in frame.iter_mut() {
        let transfer = link.start(chunk.take().unwrap(), CHUNK);
        *chunk = Some(link.finish(transfer));
    }
    link.end();
}

fn measure(name: &str, link: &mut Link, frame: &mut Frame, view: &View) {
    let n = 30;
    let start = Instant::now();
    for _ in 0..n {
        draw(frame, view, 0, 0);
    }
    let drawing = start.elapsed().as_micros() / n;
    let start = Instant::now();
    for _ in 0..n {
        flush(link, frame);
    }
    let flushing = start.elapsed().as_micros() / n;
    let start = Instant::now();
    for _ in 0..n {
        draw(frame, view, 0, 0);
        flush(link, frame);
    }
    let both = start.elapsed().as_micros() / n;
    info!("{name}: draw {drawing} us, flush {flushing} us, draw then flush {both} us");
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);
    Timer::after(Duration::from_secs(2)).await;

    let psram = Psram::new(
        peripherals.PSRAM,
        PsramConfig {
            mode: PsramMode::QuadSpi,
            ..PsramConfig::default()
        },
    );
    let (ptr, len) = psram.raw_parts();
    assert!(len >= FRAME);
    // SAFETY: mapped PSRAM that nothing else uses, for the rest of the program.
    #[allow(unsafe_code, reason = "PSRAM is raw memory")]
    let external: &'static mut [u8] = unsafe { core::slice::from_raw_parts_mut(ptr, FRAME) };

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
    link.apply_config(&config.with_frequency(Rate::from_mhz(80)));

    let internal: &'static mut [u8] = INTERNAL.take().get_mut().into_inner();
    let mut frames: [Frame; 2] = [frame!(internal), frame!(external)];
    let view = App::new(wade_core::Instant::from_millis(0), 42, Settings::DEFAULT).view();
    measure("internal SRAM", &mut link, &mut frames[0], &view);
    measure("PSRAM", &mut link, &mut frames[1], &view);

    loop {
        for (phase, name) in [(1, "internal SRAM"), (2, "PSRAM")] {
            info!("phase {phase}: {name}");
            let frame = &mut frames[phase as usize - 1];
            let end = Instant::now() + PHASE;
            let mut x = 0;
            while Instant::now() < end {
                draw(frame, &view, phase, x);
                flush(&mut link, frame);
                x = (x + 3) % 300;
            }
        }
    }
}
