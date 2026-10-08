#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Steps 6 and 7: read the FT6336 touch controller, draw targets, a dot under each touch, and a
//! text readout, and check whether the AW9523B's interrupt on GPIO 21 follows touch.

use core::fmt::Write as _;

use cores3_bringup::panel::{BLACK, GREEN, HEIGHT, Panel, RED, WHITE, WIDTH};
use cores3_bringup::power::{self, AW9523B};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c::master::{self, I2c};
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use heapless::String;
use log::info;
use wade_core::render::{Band, ROW_BYTES};

esp_bootloader_esp_idf::esp_app_desc!();

const MADCTL: u8 = 0x08;
const FT6336: u8 = 0x38;
const TARGETS: [(u16, u16); 5] = [(20, 60), (300, 60), (20, 220), (300, 220), (160, 140)];

fn crosshair(panel: &mut Panel, x: u16, y: u16, color: u16) {
    panel.fill(x - 8, y, 17, 1, color);
    panel.fill(x, y - 8, 1, 17, color);
}

fn targets(panel: &mut Panel) {
    panel.fill(0, 0, WIDTH, HEIGHT, BLACK);
    for (x, y) in TARGETS {
        crosshair(panel, x, y, RED);
    }
}

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

/// The first touch point, if any: (x, y, event flag, number of touches).
fn touch(i2c: &mut I2c<'_, esp_hal::Blocking>) -> Option<(u16, u16, u8, u8)> {
    let mut r = [0u8; 5];
    i2c.write_read(FT6336, &[0x02], &mut r).ok()?;
    let n = r[0] & 0x0F;
    if n == 0 || n > 2 {
        return None;
    }
    let x = (u16::from(r[1] & 0x0F) << 8) | u16::from(r[2]);
    let y = (u16::from(r[3] & 0x0F) << 8) | u16::from(r[4]);
    Some((x, y, r[1] >> 6, n))
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
    let int = Input::new(
        peripherals.GPIO21,
        InputConfig::default().with_pull(Pull::Up),
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

    for reg in [0xA3u8, 0xA6, 0xA8, 0xA4, 0x80, 0x88] {
        info!(
            "FT6336 reg {reg:#04x} = {:02x?}",
            power::read(&mut i2c, FT6336, reg)
        );
    }
    info!(
        "AW9523B int enable p0 {:02x?} p1 {:02x?}",
        power::read(&mut i2c, AW9523B, 0x06),
        power::read(&mut i2c, AW9523B, 0x07)
    );

    let mut buf = [0u8; ROW_BYTES * 30];
    targets(&mut panel);
    readout(&mut panel, &mut buf, "touch the red crosses");

    let mut press: Option<(u32, u32, u16, u16, u16, u16, u32)> = None;
    let mut presses = 0u32;
    loop {
        let level = int.level();
        let p1 = power::read(&mut i2c, AW9523B, 0x01).unwrap_or(0xFF);
        match (touch(&mut i2c), press.as_mut()) {
            (Some((x, y, ev, n)), None) => {
                info!("down ({x}, {y}) event {ev} touches {n}, GPIO21 {level:?}, P1 {p1:08b}");
                press = Some((u32::from(x), u32::from(y), x, x, y, y, 1));
                let _ = (ev, n);
            }
            (Some((x, y, _, _)), Some(p)) => {
                p.0 += u32::from(x);
                p.1 += u32::from(y);
                p.2 = p.2.min(x);
                p.3 = p.3.max(x);
                p.4 = p.4.min(y);
                p.5 = p.5.max(y);
                p.6 += 1;
                if x < WIDTH && y < HEIGHT {
                    panel.fill(x.saturating_sub(1), y.saturating_sub(1), 3, 3, GREEN);
                }
            }
            (None, Some(p)) => {
                let (ax, ay) = (p.0 / p.6, p.1 / p.6);
                info!(
                    "up: {} samples, mean ({ax}, {ay}), x {}..{}, y {}..{}, GPIO21 {level:?}, P1 {p1:08b}",
                    p.6, p.2, p.3, p.4, p.5
                );
                let mut s: String<40> = String::new();
                let _ = write!(s, "({ax}, {ay}) n={}", p.6);
                readout(&mut panel, &mut buf, &s);
                press = None;
                presses += 1;
                if presses % 10 == 0 {
                    targets(&mut panel);
                    crosshair(&mut panel, 160, 140, WHITE);
                }
            }
            (None, None) => {}
        }
        Timer::after(Duration::from_millis(10)).await;
    }
}
