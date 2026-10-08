//! Wade on the CYD, a stand-in for the CoreS3 Lite (docs/platforms.md#cyd-stand-in-wade-cyd).
//!
//! Runs the platform loop from docs/architecture.md#core-api: wait for a touch
//! or the next deadline, hand it to the core, carry out effects, and redraw
//! when asked.

#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

mod display;
mod touch;

use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::dma_tx_buffer;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};
use esp_hal::rng::Rng;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use wade_core::{App, Effect, Event, Instant, Settings};

use display::{BAND_BYTES, COMMAND_BYTES, Display};
use touch::TouchPanel;

esp_bootloader_esp_idf::esp_app_desc!();

/// The panel is steady with DMA at 40 MHz but not at 80 (docs/hardware-notes.md).
const DISPLAY_CLOCK: Rate = Rate::from_mhz(40);
/// The XPT2046's conversions need a slow clock.
const TOUCH_CLOCK: Rate = Rate::from_mhz(2);

/// Events waiting for the app task. Producers drop and log an event rather than
/// wait when it is full.
pub const EVENT_CAPACITY: usize = 16;
static EVENTS: Channel<CriticalSectionRawMutex, Event, EVENT_CAPACITY> = Channel::new();

/// The core's clock: milliseconds since boot.
fn now() -> Instant {
    Instant::from_millis(embassy_time::Instant::now().as_millis())
}

fn to_embassy(at: Instant) -> embassy_time::Instant {
    embassy_time::Instant::from_millis(at.as_millis())
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(DISPLAY_CLOCK)
            .with_mode(Mode::_0),
    )
    .expect("the display SPI configuration is valid")
    .with_sck(peripherals.GPIO14)
    .with_mosi(peripherals.GPIO13)
    .with_dma(peripherals.DMA_SPI2)
    .into_async();
    let mut display = Display::new(
        spi,
        dma_tx_buffer!(COMMAND_BYTES).expect("the command buffer fits DMA limits"),
        [
            dma_tx_buffer!(BAND_BYTES).expect("a band fits DMA limits"),
            dma_tx_buffer!(BAND_BYTES).expect("a band fits DMA limits"),
        ],
        Output::new(peripherals.GPIO2, Level::High, OutputConfig::default()),
        Output::new(peripherals.GPIO15, Level::High, OutputConfig::default()),
    )
    .await;

    let touch_spi = Spi::new(
        peripherals.SPI3,
        Config::default()
            .with_frequency(TOUCH_CLOCK)
            .with_mode(Mode::_0),
    )
    .expect("the touch SPI configuration is valid")
    .with_sck(peripherals.GPIO25)
    .with_mosi(peripherals.GPIO32)
    .with_miso(peripherals.GPIO39);
    let panel = TouchPanel {
        spi: touch_spi,
        cs: Output::new(peripherals.GPIO33, Level::High, OutputConfig::default()),
        irq: Input::new(peripherals.GPIO36, InputConfig::default()),
    };
    spawner.spawn(touch::run(panel, EVENTS.sender()).expect("the touch task is spawned once"));

    let rng = Rng::new();
    let seed = u64::from(rng.random()) << 32 | u64::from(rng.random());
    info!("seed {seed}");
    let mut app = App::new(now(), seed, Settings::DEFAULT);
    display.show(&app.view()).await;
    // Turned on only once the first frame is up, so the panel's power-on noise never shows.
    let _backlight = Output::new(peripherals.GPIO21, Level::High, OutputConfig::default());

    loop {
        let event = match app.next_deadline() {
            Some(deadline) => {
                match select(EVENTS.receive(), Timer::at(to_embassy(deadline))).await {
                    Either::First(event) => event,
                    Either::Second(()) => Event::deadline(now()),
                }
            }
            None => EVENTS.receive().await,
        };
        let output = app.handle(event);
        for effect in output.effects {
            match effect {
                Effect::Chime => info!("chime"),
                Effect::SaveSettings(_) => info!("settings changed; not stored yet"),
            }
        }
        if output.redraw {
            display.show(&app.view()).await;
        }
    }
}
