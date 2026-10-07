#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Step 4: raw XPT2046 touch readings, summarized per press.

use cyd_spike::panel::{BLACK, HEIGHT, Panel, RED, WHITE, WIDTH};
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
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
const TARGETS: [(u16, u16, &str); 4] = [
    (20, 20, "top-left"),
    (300, 20, "top-right"),
    (300, 220, "bottom-right"),
    (20, 220, "bottom-left"),
];
const MAX_SAMPLES: usize = 600;

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

fn crosshair(panel: &mut Panel, x: u16, y: u16, color: u16) {
    panel.fill(x - 10, y, 21, 1, color);
    panel.fill(x, y - 10, 1, 21, color);
}

fn median(values: &mut [u16]) -> u16 {
    values.sort_unstable();
    values[values.len() / 2]
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

    let mut xs = [0u16; MAX_SAMPLES];
    let mut ys = [0u16; MAX_SAMPLES];
    let mut target = 0;
    loop {
        for (i, &(x, y, _)) in TARGETS.iter().enumerate() {
            crosshair(&mut panel, x, y, if i == target { RED } else { WHITE });
        }
        let (_, _, name) = TARGETS[target];
        info!("press and hold the red crosshair: {name}");

        while irq.is_high() {
            Timer::after(Duration::from_millis(10)).await;
        }
        let start = Instant::now();
        let mut n = 0;
        let mut first = [(0u16, 0u16, 0u16, 0u16); 3];
        let mut last = [(0u16, 0u16, 0u16, 0u16); 3];
        let mut released = 0;
        while released < 3 {
            let s = touch.sample();
            if irq.is_high() {
                released += 1;
            } else {
                released = 0;
            }
            if n < first.len() {
                first[n] = s;
            }
            last.rotate_left(1);
            last[2] = s;
            if n < MAX_SAMPLES {
                xs[n] = s.0;
                ys[n] = s.1;
            }
            n += 1;
            Timer::after(Duration::from_millis(10)).await;
        }
        let kept = n.min(MAX_SAMPLES);
        let (min_x, max_x) = (xs[..kept].iter().min().unwrap(), xs[..kept].iter().max().unwrap());
        let (min_y, max_y) = (ys[..kept].iter().min().unwrap(), ys[..kept].iter().max().unwrap());
        info!(
            "{name}: {n} samples over {} ms; x {min_x}..{max_x}, y {min_y}..{max_y}",
            start.elapsed().as_millis()
        );
        info!("  median x {} y {}", median(&mut xs[..kept]), median(&mut ys[..kept]));
        info!("  first (x, y, z1, z2): {first:?}");
        info!("  last  (x, y, z1, z2): {last:?}");
        target = (target + 1) % TARGETS.len();
    }
}
