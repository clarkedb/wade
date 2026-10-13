//! The AXP2101 power rails and power key, and the AW9523B's reset, enable,
//! and interrupt lines (docs/hardware-notes.md#cores3-lite). Initialization
//! follows M5Unified.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;
use log::warn;
use wade_core::Event;
use wade_core::settings::Brightness;

use crate::{Bus, now};

const AXP2101: u8 = 0x34;
const AW9523B: u8 = 0x58;

const LDO_ENABLE: u8 = 0x90;
/// LDO enables with every rail Wade uses on except DLDO1, the backlight.
const LDOS_BACKLIGHT_OFF: u8 = 0x3F;
const DLDO1_ON: u8 = 1 << 7;
/// DLDO1's voltage: 0.5 V plus 0.1 V per step.
const DLDO1_VOLTAGE: u8 = 0x99;

const WRITE_ATTEMPTS: u32 = 3;

/// IRQ status 1. Bit 3 latches a short press of the power key, under the
/// 1 s long-press time; writing the bit back clears it.
const IRQ_STATUS_1: u8 = 0x49;
const POWER_KEY_SHORT: u8 = 1 << 3;

/// The power key reaches only the AXP2101, whose interrupt line does not reach
/// the ESP32, so its press flag is polled this often.
const KEY_POLL: Duration = Duration::from_millis(50);
/// A short press that turns the device on ends within a second of power-on.
/// The flag is cleared once that has passed, so the press cannot put Wade to
/// sleep as he starts.
const POWER_ON_PRESS: Duration = Duration::from_secs(1);

const AW_INPUT_P0: u8 = 0x00;
const AW_INPUT_P1: u8 = 0x01;
const AW_OUTPUT_P0: u8 = 0x02;
const AW_OUTPUT_P1: u8 = 0x03;
/// Port 1, bit 1: the LCD's reset, active low.
const LCD_RESET: u8 = 1 << 1;
/// Port 1, bit 2: the touch controller's interrupt, low while touched.
const TOUCH_INT: u8 = 1 << 2;

/// The backlight's DLDO1 step for each level. Below step 22 the screen is
/// dark; above 28 is out of range.
#[must_use]
pub const fn backlight_step(brightness: Brightness) -> u8 {
    match brightness {
        Brightness::Quarter => 23,
        Brightness::Half => 24,
        Brightness::ThreeQuarters => 26,
        Brightness::Full => 28,
    }
}

/// Write a register, trying again on failure: the first transaction after a
/// reset sometimes goes unacknowledged.
async fn write<B: I2c>(bus: &mut B, addr: u8, reg: u8, value: u8) -> Result<(), B::Error> {
    let mut result = bus.write(addr, &[reg, value]).await;
    for _ in 1..WRITE_ATTEMPTS {
        if result.is_ok() {
            break;
        }
        Timer::after(Duration::from_millis(1)).await;
        result = bus.write(addr, &[reg, value]).await;
    }
    result
}

async fn read<B: I2c>(bus: &mut B, addr: u8, reg: u8) -> Result<u8, B::Error> {
    let mut b = [0];
    bus.write_read(addr, &[reg], &mut b).await?;
    Ok(b[0])
}

/// Turn on the rails, release the resets for the display, touch, and amplifier,
/// pulse the LCD's reset, and leave the backlight off. The PMIC keeps its
/// registers across a reset, so every rail Wade relies on is set here.
pub async fn init<B: I2c>(bus: &mut B) -> Result<(), B::Error> {
    for (reg, value) in [
        (LDO_ENABLE, LDOS_BACKLIGHT_OFF),
        (0x92, 13),   // ALDO1 1.8 V: amplifier
        (0x93, 28),   // ALDO2 3.3 V: microphone codec
        (0x94, 28),   // ALDO3 3.3 V: camera
        (0x95, 28),   // ALDO4 3.3 V: SD card
        (0x27, 0x00), // power key: hold 1 s to turn on, 4 s to turn off
        (0x69, 0x11), // charge LED
        (0x10, 0x30), // PMU common config
        (0x30, 0x0F), // ADCs on
    ] {
        write(bus, AXP2101, reg, value).await?;
    }

    // Port 0: touch reset (bit 0) and amplifier reset (bit 2) high. Port 1:
    // camera reset (bit 0), LCD reset (bit 1), and boost enable (bit 7) high.
    let p0 = read(bus, AW9523B, AW_OUTPUT_P0).await?;
    write(bus, AW9523B, AW_OUTPUT_P0, p0 | 0b0000_0101).await?;
    let p1 = read(bus, AW9523B, AW_OUTPUT_P1).await? | 0b1000_0011;
    write(bus, AW9523B, AW_OUTPUT_P1, p1).await?;
    for (reg, value) in [
        (0x04, 0b0001_1000), // port 0 direction: bits 3 and 4 in
        (0x05, 0b0000_1100), // port 1 direction: touch and amplifier interrupts in
        (0x06, 0xFF),        // port 0 interrupts off
        (0x07, !TOUCH_INT),  // port 1 interrupts: touch only
        (0x11, 0b0001_0000), // port 0 push-pull
        (0x12, 0xFF),        // port 0 GPIO mode, not LED
        (0x13, 0xFF),        // port 1 GPIO mode, not LED
    ] {
        write(bus, AW9523B, reg, value).await?;
    }

    write(bus, AW9523B, AW_OUTPUT_P1, p1 & !LCD_RESET).await?;
    Timer::after(Duration::from_millis(10)).await;
    write(bus, AW9523B, AW_OUTPUT_P1, p1).await?;
    Timer::after(Duration::from_millis(120)).await;
    Ok(())
}

/// Set the backlight's level, turning it on if it was off.
pub async fn set_brightness<B: I2c>(bus: &mut B, brightness: Brightness) -> Result<(), B::Error> {
    write(bus, AXP2101, DLDO1_VOLTAGE, backlight_step(brightness)).await?;
    write(bus, AXP2101, LDO_ENABLE, LDOS_BACKLIGHT_OFF | DLDO1_ON).await
}

/// Whether the touch controller's interrupt shows a touch. Reading both of
/// the AW9523B's input ports also releases its own interrupt line, whichever
/// pin raised it.
pub async fn touch_held<B: I2c>(bus: &mut B) -> Result<bool, B::Error> {
    read(bus, AW9523B, AW_INPUT_P0).await?;
    Ok(read(bus, AW9523B, AW_INPUT_P1).await? & TOUCH_INT == 0)
}

/// Whether the power key had a short press since the last call. Clears the flag.
async fn take_short_press<B: I2c>(bus: &mut B) -> Result<bool, B::Error> {
    if read(bus, AXP2101, IRQ_STATUS_1).await? & POWER_KEY_SHORT == 0 {
        return Ok(false);
    }
    write(bus, AXP2101, IRQ_STATUS_1, POWER_KEY_SHORT).await?;
    Ok(true)
}

/// Sends a `PowerButton` event for each short press of the power key.
#[embassy_executor::task]
pub async fn run(
    mut bus: Bus,
    events: Sender<'static, CriticalSectionRawMutex, Event, { crate::EVENT_CAPACITY }>,
) {
    Timer::at(embassy_time::Instant::MIN + POWER_ON_PRESS).await;
    // The first read only clears the flag (see `POWER_ON_PRESS`).
    let mut first = true;
    let mut failing = false;
    loop {
        match take_short_press(&mut bus).await {
            Ok(pressed) => {
                if pressed && !first && events.try_send(Event::power_button(now())).is_err() {
                    warn!("event queue full; dropped a power button press");
                }
                first = false;
                failing = false;
            }
            // Logged once, not on every poll, while the bus keeps failing.
            Err(e) => {
                if !failing {
                    warn!("power key unreadable: {e:?}");
                }
                failing = true;
            }
        }
        Timer::after(KEY_POLL).await;
    }
}
