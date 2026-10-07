#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Step 5: Wade's chime through the speaker on GPIO 26, two ways. Plays each
//! twice, then stops; reset to hear them again.

use core::f32::consts::TAU;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::analog::dac::Dac;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::DriveMode;
use esp_hal::ledc::channel::{self, ChannelIFace};
use esp_hal::ledc::timer::{self, TimerIFace};
use esp_hal::ledc::{LSGlobalClkSource, Ledc, LowSpeed};
use esp_hal::time::{Instant, Rate};
use esp_hal::timer::timg::TimerGroup;
use log::info;
use wade_core::sound::CHIME;

esp_bootloader_esp_idf::esp_app_desc!();

const SAMPLE_RATE: u32 = 16_000;
const SAMPLES: usize = 12_000; // the chime's 750 ms
const OVERTONE: f32 = 0.2;
const ATTACK: f32 = 0.004;
const RELEASE: f32 = 0.006;

/// The desktop's bell synthesis, as unsigned 8-bit samples centered on 128.
fn synthesize(out: &mut [u8; SAMPLES]) -> usize {
    let mut n = 0;
    for tone in CHIME {
        let length = tone.duration.as_millis() as f32 / 1000.0;
        let decay = length / 4.0;
        let hz = f32::from(tone.hz);
        let count = (length * SAMPLE_RATE as f32) as usize;
        for i in 0..count {
            let t = i as f32 / SAMPLE_RATE as f32;
            let envelope = (t / ATTACK).min(1.0)
                * ((length - t) / RELEASE).min(1.0)
                * libm::expf(-t / decay);
            let wave = libm::sinf(TAU * hz * t) + OVERTONE * libm::sinf(2.0 * TAU * hz * t);
            let s = envelope * wave / (1.0 + OVERTONE);
            out[n] = (128.0 + 127.0 * s) as u8;
            n += 1;
        }
    }
    n
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let mut peripherals =
        esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // 1: PWM square waves, with a hardware duty fade for the decay.
    let mut ledc = Ledc::new(peripherals.LEDC);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    for round in 1..=2 {
        info!("1. PWM square waves, round {round}");
        for tone in CHIME {
            let mut t = ledc.timer::<LowSpeed>(timer::Number::Timer0);
            t.configure(timer::config::Config {
                duty: timer::config::Duty::Duty10Bit,
                clock_source: timer::LSClockSource::APBClk,
                frequency: Rate::from_hz(u32::from(tone.hz)),
            })
            .unwrap();
            let mut ch = ledc.channel(channel::Number::Channel0, peripherals.GPIO26.reborrow());
            ch.configure(channel::config::Config {
                timer: &t,
                duty_pct: 50,
                drive_mode: DriveMode::PushPull,
            })
            .unwrap();
            let ms = tone.duration.as_millis() as u16;
            ch.start_duty_fade(50, 0, ms).unwrap();
            Timer::after(Duration::from_millis(u64::from(ms))).await;
            ch.set_duty(0).unwrap();
        }
        Timer::after(Duration::from_secs(2)).await;
    }

    // 2: DAC samples of the desktop's bell sound, timed by busy-waiting.
    let mut samples = [128u8; SAMPLES];
    let count = synthesize(&mut samples);
    let mut dac = Dac::new(peripherals.DAC2, peripherals.GPIO26);
    dac.write(128);
    Timer::after(Duration::from_millis(500)).await;
    for round in 1..=2 {
        info!("2. DAC sine bell, round {round}");
        let start = Instant::now();
        for (i, &s) in samples[..count].iter().enumerate() {
            let due = start + esp_hal::time::Duration::from_micros(i as u64 * 1_000_000 / u64::from(SAMPLE_RATE));
            while Instant::now() < due {}
            dac.write(s);
        }
        dac.write(128);
        Timer::after(Duration::from_secs(2)).await;
    }

    info!("done; press reset to hear them again");
    loop {
        Timer::after(Duration::from_secs(60)).await;
    }
}
