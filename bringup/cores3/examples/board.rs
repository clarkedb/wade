#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Steps 1 and 2: log over USB serial, list the parts on the internal I²C bus, and check the
//! battery and PSRAM.

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::i2c::master::{Config, I2c};
use esp_hal::psram::{Psram, PsramConfig, PsramMode};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

/// Addresses M5Unified expects on the CoreS3's internal bus.
const KNOWN: &[(u8, &str)] = &[
    (0x21, "GC0308 camera"),
    (0x23, "LTR-553ALS proximity and light"),
    (0x34, "AXP2101 PMIC"),
    (0x36, "AW88298 amplifier"),
    (0x38, "FT6336 touch"),
    (0x40, "ES7210 microphone codec"),
    (0x51, "BM8563 clock"),
    (0x58, "AW9523B IO expander"),
    (0x69, "BMI270 IMU"),
];

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // Give the serial monitor time to attach.
    Timer::after(Duration::from_secs(2)).await;

    let mut i2c = I2c::new(
        peripherals.I2C0,
        Config::default().with_frequency(Rate::from_khz(100)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO12)
    .with_scl(peripherals.GPIO11);

    let psram = Psram::new(
        peripherals.PSRAM,
        PsramConfig {
            mode: PsramMode::QuadSpi,
            ..PsramConfig::default()
        },
    );
    let (ptr, len) = psram.raw_parts();
    info!("PSRAM: {len} bytes at {ptr:p}");
    if len > 0 {
        // SAFETY: `raw_parts` is mapped PSRAM that nothing else uses.
        #[allow(unsafe_code, reason = "PSRAM is raw memory")]
        let ram = unsafe { core::slice::from_raw_parts_mut(ptr.cast::<u32>(), len / 4) };
        for (i, word) in ram.iter_mut().enumerate() {
            *word = (i as u32).wrapping_mul(0x9E37_79B9);
        }
        let bad = ram
            .iter()
            .enumerate()
            .filter(|(i, w)| **w != (*i as u32).wrapping_mul(0x9E37_79B9))
            .count();
        info!(
            "PSRAM write and read back: {bad} bad words of {}",
            ram.len()
        );
    }

    let mut reg = |r: u8| {
        let mut b = [0u8];
        i2c.write_read(0x34, &[r], &mut b).map(|()| b[0])
    };
    // AXP2101: 0x03 chip ID, 0x00 status 1 (bit 5 VBUS good, bit 3 battery present),
    // 0x01 status 2 (bits 6..5 charge direction), 0xA4 battery percent.
    info!(
        "AXP2101 id {:02x?}, status {:02x?} {:02x?}, battery {:?}%",
        reg(0x03),
        reg(0x00),
        reg(0x01),
        reg(0xA4)
    );

    loop {
        info!("scanning the internal I2C bus");
        let mut found = 0;
        for addr in 0x08..0x78u8 {
            if i2c.read(addr, &mut [0u8; 1]).is_ok() {
                found += 1;
                let name = KNOWN
                    .iter()
                    .find(|(a, _)| *a == addr)
                    .map_or("unknown", |(_, n)| *n);
                info!("  0x{addr:02x}  {name}");
            }
        }
        for (addr, name) in KNOWN {
            let mut buf = [0u8; 1];
            if i2c.read(*addr, &mut buf).is_err() {
                info!("  missing: 0x{addr:02x}  {name}");
            }
        }
        info!("{found} devices");
        Timer::after(Duration::from_secs(5)).await;
    }
}
