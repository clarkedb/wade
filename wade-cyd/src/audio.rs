//! The chime through the amp on GPIO 26 (docs/hardware-notes.md#audio).
//!
//! Each note is a PWM square wave, the loudest signal the board's amp gets from
//! the pin. It holds at the volume's duty, then fades in hardware, so the CPU
//! only wakes between notes.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::DriveMode;
use esp_hal::ledc::LowSpeed;
use esp_hal::ledc::channel::{self, ChannelIFace};
use esp_hal::ledc::timer::{self, TimerIFace};
use esp_hal::peripherals::GPIO26;
use esp_hal::time::Rate;
use log::warn;
use wade_core::settings::Volume;
use wade_core::sound::{CHIME, Tone};

/// A chime request from the app task, at the volume when it fell due.
#[derive(Clone, Copy, Debug)]
pub struct Chime(pub Volume);

/// The duty for `volume`. A pulse wave's fundamental scales with sin(π × duty),
/// so 50% is loudest, and 17% and 8% halve it each step, as on desktop.
const fn duty(volume: Volume) -> Option<u8> {
    match volume {
        Volume::Off => None,
        Volume::Soft => Some(8),
        Volume::Medium => Some(17),
        Volume::Loud => Some(50),
    }
}

pub const CHIME_CAPACITY: usize = 2;

#[derive(Debug)]
#[expect(dead_code, reason = "read only by the warning that logs it")]
enum Error {
    Timer(timer::Error),
    Channel(channel::Error),
}

impl From<timer::Error> for Error {
    fn from(e: timer::Error) -> Self {
        Error::Timer(e)
    }
}

impl From<channel::Error> for Error {
    fn from(e: channel::Error) -> Self {
        Error::Channel(e)
    }
}

async fn play(
    tone: &Tone,
    duty: u8,
    pwm: &mut timer::Timer<'static, LowSpeed>,
    pin: &mut GPIO26<'static>,
) -> Result<(), Error> {
    let ms = u16::try_from(tone.duration.as_millis()).unwrap_or(u16::MAX);
    pwm.configure(timer::config::Config {
        duty: timer::config::Duty::Duty10Bit,
        clock_source: timer::LSClockSource::APBClk,
        frequency: Rate::from_hz(u32::from(tone.hz)),
    })?;
    let mut out = channel::Channel::new(channel::Number::Channel1, pin.reborrow());
    out.configure(channel::config::Config {
        timer: &*pwm,
        duty_pct: duty,
        drive_mode: DriveMode::PushPull,
    })?;
    let hold = ms / 2;
    Timer::after(Duration::from_millis(u64::from(hold))).await;
    out.start_duty_fade(duty, 0, ms - hold)?;
    Timer::after(Duration::from_millis(u64::from(ms - hold))).await;
    out.set_duty(0)?;
    Ok(())
}

#[embassy_executor::task]
pub async fn run(
    mut pwm: timer::Timer<'static, LowSpeed>,
    mut pin: GPIO26<'static>,
    requests: Receiver<'static, CriticalSectionRawMutex, Chime, CHIME_CAPACITY>,
) {
    loop {
        let Chime(volume) = requests.receive().await;
        let Some(duty) = duty(volume) else { continue };
        for tone in &CHIME {
            if let Err(e) = play(tone, duty, &mut pwm, &mut pin).await {
                warn!("chime not played: {e:?}");
                break;
            }
        }
    }
}
