//! Wade on the M5Stack CoreS3 Lite (docs/platforms.md#m5stack-cores3-lite-wade-cores3).
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

mod audio;
mod display;
#[cfg(feature = "measure")]
mod measure;
mod power;
mod storage;
mod touch;

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::Async;
use esp_hal::clock::CpuClock;
use esp_hal::dma_tx_buffer;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::I2c;
use esp_hal::i2s::master::{Channels, DataFormat, I2s, TdmConfig};
use esp_hal::rng::Rng;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::{info, warn};
use static_cell::StaticCell;
use wade_core::{App, Effect, Event, Instant, Settings};

use audio::{CHIME_BYTES, CHIME_CAPACITY, Chime};
use display::{BAND_BYTES, COMMAND_BYTES, Display};

esp_bootloader_esp_idf::esp_app_desc!();

/// The panel is steady with DMA at 80 MHz (docs/hardware-notes.md#cores3-lite).
const DISPLAY_CLOCK: Rate = Rate::from_mhz(80);
const I2C_CLOCK: Rate = Rate::from_khz(400);

/// Events waiting for the app task. Producers drop and log an event rather than
/// wait when it is full.
pub const EVENT_CAPACITY: usize = 16;
static EVENTS: Channel<CriticalSectionRawMutex, Event, EVENT_CAPACITY> = Channel::new();
static CHIMES: Channel<CriticalSectionRawMutex, Chime, CHIME_CAPACITY> = Channel::new();

/// The internal I²C bus, shared by the PMIC, IO expander, touch controller,
/// and amplifier.
static I2C: StaticCell<Mutex<CriticalSectionRawMutex, I2c<'static, Async>>> = StaticCell::new();
pub type Bus = I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>;

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

    let i2c = I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(I2C_CLOCK),
    )
    .expect("the I2C configuration is valid")
    .with_sda(peripherals.GPIO12)
    .with_scl(peripherals.GPIO11)
    .into_async();
    let i2c = I2C.init(Mutex::new(i2c));
    let mut bus = Bus::new(i2c);
    if let Err(e) = power::init(&mut bus).await {
        warn!("power setup failed: {e:?}");
    }

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(DISPLAY_CLOCK)
            .with_mode(Mode::_0),
    )
    .expect("the display SPI configuration is valid")
    .with_sck(peripherals.GPIO36)
    .with_mosi(peripherals.GPIO37)
    .with_dma(peripherals.DMA_CH0)
    .into_async();
    let mut display = Display::new(
        spi,
        dma_tx_buffer!(COMMAND_BYTES).expect("the command buffer fits DMA limits"),
        [
            dma_tx_buffer!(BAND_BYTES).expect("a band fits DMA limits"),
            dma_tx_buffer!(BAND_BYTES).expect("a band fits DMA limits"),
        ],
        Output::new(peripherals.GPIO35, Level::High, OutputConfig::default()),
        Output::new(peripherals.GPIO3, Level::High, OutputConfig::default()),
    )
    .await;

    let interrupt = Input::new(
        peripherals.GPIO21,
        InputConfig::default().with_pull(Pull::Up),
    );
    spawner.spawn(
        touch::run(Bus::new(i2c), interrupt, EVENTS.sender())
            .expect("the touch task is spawned once"),
    );

    let i2s = I2s::new(
        peripherals.I2S1,
        peripherals.DMA_CH1,
        TdmConfig::new_tdm_philips()
            .with_sample_rate(Rate::from_hz(audio::SAMPLE_RATE))
            .with_data_format(DataFormat::Data16Channel16)
            .with_channels(Channels::STEREO),
    )
    .expect("the I2S configuration is valid")
    .into_async();
    let tx = i2s
        .i2s_tx
        .with_bclk(peripherals.GPIO34)
        .with_ws(peripherals.GPIO33)
        .with_dout(peripherals.GPIO13)
        .build();
    spawner.spawn(
        audio::run(
            Bus::new(i2c),
            tx,
            dma_tx_buffer!(CHIME_BYTES).expect("the chime fits DMA limits"),
            CHIMES.receiver(),
        )
        .expect("the audio task is spawned once"),
    );

    let rng = Rng::new();
    let seed = u64::from(rng.random()) << 32 | u64::from(rng.random());
    info!("seed {seed}");
    let mut store = storage::open(peripherals.FLASH);
    let settings = match &mut store {
        Some(store) => store.load().await.unwrap_or_else(|e| {
            warn!("settings unreadable ({e:?}); starting with the defaults");
            Settings::DEFAULT
        }),
        None => Settings::DEFAULT,
    };
    let mut app = App::new(now(), seed, settings);
    display.show(&app.view()).await;
    // Turned on only once the first frame is up, so the panel's power-on noise never shows.
    if let Err(e) = power::set_brightness(&mut bus, settings.brightness()).await {
        warn!("backlight not set: {e:?}");
    }

    #[cfg(feature = "measure")]
    let mut stats = {
        spawner.spawn(measure::uptime().expect("the uptime task is spawned once"));
        measure::Stats::new()
    };

    let mut redraw = false;
    #[cfg(feature = "measure")]
    let mut touched = None;
    loop {
        let event = match app.next_deadline() {
            Some(deadline) => {
                match select(EVENTS.receive(), Timer::at(to_embassy(deadline))).await {
                    Either::First(event) => event,
                    Either::Second(()) => {
                        #[cfg(feature = "measure")]
                        stats.deadline(to_embassy(deadline), embassy_time::Instant::now());
                        Event::deadline(now())
                    }
                }
            }
            None => EVENTS.receive().await,
        };
        let output = app.handle(event);
        for effect in output.effects {
            match effect {
                Effect::Chime => {
                    if CHIMES.try_send(Chime).is_err() {
                        warn!("chime queue full; dropped a chime");
                    }
                }
                Effect::SetBrightness(brightness) => {
                    if let Err(e) = power::set_brightness(&mut bus, brightness).await {
                        warn!("backlight not set: {e:?}");
                    }
                }
                Effect::SaveSettings(settings) => {
                    if let Some(store) = &mut store
                        && let Err(e) = store.save(&settings).await
                    {
                        warn!("settings not saved: {e:?}");
                    }
                }
            }
        }
        redraw |= output.redraw;
        #[cfg(feature = "measure")]
        if output.redraw && matches!(event.kind, wade_core::EventKind::Touch(_)) {
            touched.get_or_insert(to_embassy(event.at));
        }
        // Handle every event already waiting before drawing, so a burst of
        // touches costs one frame rather than queueing a frame each.
        if redraw && EVENTS.is_empty() {
            redraw = false;
            #[cfg(feature = "measure")]
            let started = embassy_time::Instant::now();
            display.show(&app.view()).await;
            #[cfg(feature = "measure")]
            stats.frame(started, embassy_time::Instant::now(), touched.take());
        }
    }
}
