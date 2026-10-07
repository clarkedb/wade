#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Step 4: touch with calibration and filtering. Draws a dot under each touch.

use cyd_spike::panel::{BLACK, HEIGHT, Panel, WHITE, WIDTH};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::Blocking;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

const MADCTL: u8 = 0x28;
/// Below this pressure (z1) a sample is noise, not a touch.
const MIN_PRESSURE: u16 = 60;

/// Raw readings at the screen's edges, from the calibration run. The
/// controller's Y runs along the screen's x axis and its X along the y axis.
const LEFT: i32 = 166;
const RIGHT: i32 = 3744;
const TOP: i32 = 272;
const BOTTOM: i32 = 3908;

fn to_screen(raw_x: u16, raw_y: u16) -> (u16, u16) {
    let x = (i32::from(raw_y) - LEFT) * 320 / (RIGHT - LEFT);
    let y = (i32::from(raw_x) - TOP) * 240 / (BOTTOM - TOP);
    (x.clamp(0, 319) as u16, y.clamp(0, 239) as u16)
}

fn median3(mut v: [u16; 3]) -> u16 {
    v.sort_unstable();
    v[1]
}

struct Touch<'d> {
    spi: Spi<'d, Blocking>,
    cs: Output<'d>,
}

impl Touch<'_> {
    /// One 12-bit conversion for control byte `cmd`.
    fn read(&mut self, cmd: u8) -> u16 {
        let mut buf = [cmd, 0, 0];
        self.cs.set_low();
        self.spi.transfer(&mut buf).unwrap();
        self.cs.set_high();
        (u16::from_be_bytes([buf[1], buf[2]]) >> 3) & 0x0FFF
    }

    /// (x, y, z1, z2), in the controller's own axes.
    fn sample(&mut self) -> (u16, u16, u16, u16) {
        let z1 = self.read(0xB1);
        let z2 = self.read(0xC1);
        let x = self.read(0xD1);
        let y = self.read(0x91);
        let _ = self.read(0xD0); // power down, re-enabling the IRQ line
        (x, y, z1, z2)
    }
}

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
    let _backlight = Output::new(peripherals.GPIO21, Level::High, OutputConfig::default());
    panel.init(MADCTL, false).await;
    panel.fill(0, 0, WIDTH, HEIGHT, BLACK);

    let tp = Config::default()
        .with_frequency(Rate::from_mhz(2))
        .with_mode(Mode::_0);
    let mut touch = Touch {
        spi: Spi::new(peripherals.SPI3, tp)
            .unwrap()
            .with_sck(peripherals.GPIO25)
            .with_mosi(peripherals.GPIO32)
            .with_miso(peripherals.GPIO39),
        cs: Output::new(peripherals.GPIO33, Level::High, OutputConfig::default()),
    };
    let irq = Input::new(peripherals.GPIO36, InputConfig::default());

    let mut touches = 0u32;
    loop {
        if irq.is_high() {
            Timer::after(Duration::from_millis(10)).await;
            continue;
        }
        // Three samples, all with real pressure, or none.
        let samples = [touch.sample(), touch.sample(), touch.sample()];
        if irq.is_low() && samples.iter().all(|s| s.2 >= MIN_PRESSURE) {
            let raw_x = median3(samples.map(|s| s.0));
            let raw_y = median3(samples.map(|s| s.1));
            let (x, y) = to_screen(raw_x, raw_y);
            panel.fill(x.saturating_sub(1).min(317), y.saturating_sub(1).min(237), 3, 3, WHITE);
            touches += 1;
            if touches % 50 == 0 {
                info!("{touches} points; last at ({x}, {y})");
            }
        }
        Timer::after(Duration::from_millis(10)).await;
    }
}
