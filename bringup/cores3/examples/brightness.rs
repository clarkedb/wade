#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Step 9: backlight brightness through the AXP2101's DLDO1 voltage, with Wade on screen. Steps
//! down through the voltages, fades, then turns the rail off and on.

use core::fmt::Write as _;

use cores3_bringup::panel::{Panel, WIDTH};
use cores3_bringup::power::{self, AXP2101};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{self, I2c};
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use heapless::String;
use log::info;
use wade_core::render::{Band, ROW_BYTES};
use wade_core::{App, Settings};

esp_bootloader_esp_idf::esp_app_desc!();

const MADCTL: u8 = 0x08;
const ROWS: usize = 40;
/// DLDO1 voltage: 0.5 V + 0.1 V per step.
const DLDO1_VOLTAGE: u8 = 0x99;
const LDO_ENABLE: u8 = 0x90;
const DLDO1_ON: u8 = 1 << 7;

/// Show `text` in the top 30 rows.
fn readout(panel: &mut Panel, buf: &mut [u8], text: &str) {
    let mut band = Band::new(buf, 0);
    let Ok(()) = band.clear(Rgb565::BLACK);
    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    let Ok(_) = Text::new(text, Point::new(4, 20), style).draw(&mut band);
    panel.begin(0, 0, WIDTH - 1, 29);
    panel.spi.write(&band.bytes()[..ROW_BYTES * 30]).unwrap();
    panel.end();
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
    info!(
        "DLDO1 at boot: {:02x?}",
        power::read(&mut i2c, AXP2101, DLDO1_VOLTAGE)
    );

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
    panel.init(MADCTL, true).await;
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

    loop {
        for step in (10..=28u8).rev().step_by(2) {
            let mv = 500 + 100 * u32::from(step);
            info!("DLDO1 step {step}: {mv} mV");
            let mut s: String<32> = String::new();
            let _ = write!(s, "DLDO1 {step}: {mv} mV");
            readout(&mut panel, &mut buf, &s);
            power::write(&mut i2c, AXP2101, DLDO1_VOLTAGE, step).unwrap();
            Timer::after(Duration::from_millis(2500)).await;
        }

        info!("fade 28 -> 20 -> 28, 60 ms a step");
        readout(&mut panel, &mut buf, "fade");
        for _ in 0..2 {
            for step in (20..=28u8).rev().chain(20..=28) {
                power::write(&mut i2c, AXP2101, DLDO1_VOLTAGE, step).unwrap();
                Timer::after(Duration::from_millis(60)).await;
            }
        }

        info!("DLDO1 off for 2 s");
        let enable = power::read(&mut i2c, AXP2101, LDO_ENABLE).unwrap();
        power::write(&mut i2c, AXP2101, LDO_ENABLE, enable & !DLDO1_ON).unwrap();
        Timer::after(Duration::from_secs(2)).await;
        power::write(&mut i2c, AXP2101, LDO_ENABLE, enable | DLDO1_ON).unwrap();
        readout(&mut panel, &mut buf, "back on");
        Timer::after(Duration::from_secs(2)).await;
    }
}
