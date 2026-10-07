#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Step 6: backlight brightness by PWM on GPIO 21, with Wade on screen.

use cyd_bringup::panel::Panel;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{DriveMode, Level, Output, OutputConfig};
use esp_hal::ledc::channel::{self, ChannelIFace};
use esp_hal::ledc::timer::{self, TimerIFace};
use esp_hal::ledc::{LSGlobalClkSource, Ledc, LowSpeed};
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use wade_core::render::{Band, ROW_BYTES};
use wade_core::{App, Settings};

esp_bootloader_esp_idf::esp_app_desc!();

const MADCTL: u8 = 0x28;
const ROWS: usize = 40;

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let lcd = Config::default()
        .with_frequency(Rate::from_mhz(40))
        .with_mode(Mode::_0);
    let spi = Spi::new(peripherals.SPI2, lcd)
        .unwrap()
        .with_sck(peripherals.GPIO14)
        .with_mosi(peripherals.GPIO13)
        .with_miso(peripherals.GPIO12);
    let mut panel = Panel {
        spi,
        dc: Output::new(peripherals.GPIO2, Level::High, OutputConfig::default()),
        cs: Output::new(peripherals.GPIO15, Level::High, OutputConfig::default()),
    };
    panel.init(MADCTL, false).await;
    let view = App::new(wade_core::Instant::from_millis(0), 42, Settings::DEFAULT).view();
    let mut buf = [0u8; ROW_BYTES * ROWS];
    panel.frame(
        &mut buf,
        |b, top| {
            let mut band = Band::new(b, top);
            band.draw(&view);
            band.bytes().len()
        },
        ROWS,
    );

    let mut ledc = Ledc::new(peripherals.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let mut t = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    t.configure(timer::config::Config {
        duty: timer::config::Duty::Duty10Bit,
        clock_source: timer::LSClockSource::APBClk,
        frequency: Rate::from_khz(5),
    })
    .unwrap();
    let mut backlight = ledc.channel(channel::Number::Channel0, peripherals.GPIO21);
    backlight
        .configure(channel::config::Config {
            timer: &t,
            duty_pct: 100,
            drive_mode: DriveMode::PushPull,
        })
        .unwrap();

    loop {
        for pct in [25, 50, 75, 100] {
            info!("backlight {pct}%");
            backlight.set_duty(pct).unwrap();
            Timer::after(Duration::from_secs(2)).await;
        }
        info!("fade 100% to 5% and back");
        backlight.start_duty_fade(100, 5, 1500).unwrap();
        Timer::after(Duration::from_millis(1600)).await;
        backlight.start_duty_fade(5, 100, 1500).unwrap();
        Timer::after(Duration::from_millis(1600)).await;
    }
}
