#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Steps 3 and 4: power up the display, fill it with solid colors, mark three corners, and time
//! full frames.

use cores3_bringup::panel::{BLACK, BLUE, GREEN, HEIGHT, Panel, RED, WHITE, WIDTH};
use cores3_bringup::power;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{self, I2c};
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

/// Landscape, BGR, with inversion on: confirmed on this board in step 4.
const MADCTL: u8 = 0x08;
const INVERT: bool = true;
const ROWS: usize = 40;

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
    info!("power on");

    let spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO36)
    .with_mosi(peripherals.GPIO37);
    let mut panel = Panel {
        spi,
        dc: Output::new(peripherals.GPIO35, Level::High, OutputConfig::default()),
        cs: Output::new(peripherals.GPIO3, Level::High, OutputConfig::default()),
    };
    panel.init(MADCTL, INVERT).await;
    info!("panel on: MADCTL {MADCTL:#04x}, invert {INVERT}");

    let mut buf = [0u8; WIDTH as usize * 2 * ROWS];
    loop {
        for (name, color) in [
            ("red", RED),
            ("green", GREEN),
            ("blue", BLUE),
            ("white", WHITE),
        ] {
            info!("fill {name}");
            panel.fill(0, 0, WIDTH, HEIGHT, color);
            Timer::after(Duration::from_secs(2)).await;
        }
        info!("corners: red top left, green top right, blue bottom left");
        panel.fill(0, 0, WIDTH, HEIGHT, BLACK);
        panel.fill(0, 0, 40, 40, RED);
        panel.fill(WIDTH - 40, 0, 40, 40, GREEN);
        panel.fill(0, HEIGHT - 40, 40, 40, BLUE);
        Timer::after(Duration::from_secs(4)).await;

        buf.fill(0x55);
        let frames = 30;
        let start = Instant::now();
        for _ in 0..frames {
            panel.frame(&mut buf, |b, _| b.len(), ROWS);
        }
        info!(
            "full frame, {ROWS}-row bands, blocking SPI at 40 MHz: {} us",
            start.elapsed().as_micros() / frames
        );
    }
}
