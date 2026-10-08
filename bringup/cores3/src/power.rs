//! Power rails (AXP2101) and reset and enable lines (AW9523B), following M5Unified's CoreS3 setup.

use embassy_time::{Duration, Timer};
use esp_hal::Blocking;
use esp_hal::i2c::master::{Error, I2c};

pub const AXP2101: u8 = 0x34;
pub const AW9523B: u8 = 0x58;

/// AW9523B port 1, bit 1: the LCD's reset line, active low.
const LCD_RST: u8 = 1 << 1;

pub fn write(i2c: &mut I2c<'_, Blocking>, addr: u8, reg: u8, value: u8) -> Result<(), Error> {
    i2c.write(addr, &[reg, value])
}

pub fn read(i2c: &mut I2c<'_, Blocking>, addr: u8, reg: u8) -> Result<u8, Error> {
    let mut b = [0u8];
    i2c.write_read(addr, &[reg], &mut b).map(|()| b[0])
}

fn modify(
    i2c: &mut I2c<'_, Blocking>,
    addr: u8,
    reg: u8,
    f: impl FnOnce(u8) -> u8,
) -> Result<(), Error> {
    let v = read(i2c, addr, reg)?;
    write(i2c, addr, reg, f(v))
}

/// Turn on the rails and release the resets for the display, touch, and amplifier, then
/// pulse the LCD's reset.
pub async fn init(i2c: &mut I2c<'_, Blocking>) -> Result<(), Error> {
    for (reg, value) in [
        (0x90, 0xBF), // LDO enables: ALDO1-4, BLDO1-2, DLDO1 (backlight)
        (0x92, 13),   // ALDO1 1.8 V: AW88298 amplifier
        (0x93, 28),   // ALDO2 3.3 V: ES7210 microphone codec
        (0x94, 28),   // ALDO3 3.3 V: camera
        (0x95, 28),   // ALDO4 3.3 V: SD card
        (0x27, 0x00), // power key: hold 1 s to turn on, 4 s to turn off
        (0x69, 0x11), // charge LED
        (0x10, 0x30), // PMU common config
        (0x30, 0x0F), // ADCs on
    ] {
        write(i2c, AXP2101, reg, value)?;
    }

    // Port 0: touch reset (bit 0) and amplifier reset (bit 2) high. Port 1: camera reset
    // (bit 0), LCD reset (bit 1), and boost enable (bit 7) high.
    modify(i2c, AW9523B, 0x02, |v| v | 0b0000_0101)?;
    modify(i2c, AW9523B, 0x03, |v| v | 0b1000_0011)?;
    write(i2c, AW9523B, 0x04, 0b0001_1000)?; // port 0 direction: bits 3, 4 inputs
    write(i2c, AW9523B, 0x05, 0b0000_1100)?; // port 1 direction: touch and amplifier interrupts in
    write(i2c, AW9523B, 0x11, 0b0001_0000)?; // port 0 push-pull
    write(i2c, AW9523B, 0x12, 0xFF)?; // port 0 GPIO mode, not LED
    write(i2c, AW9523B, 0x13, 0xFF)?; // port 1 GPIO mode, not LED

    modify(i2c, AW9523B, 0x03, |v| v & !LCD_RST)?;
    Timer::after(Duration::from_millis(10)).await;
    modify(i2c, AW9523B, 0x03, |v| v | LCD_RST)?;
    Timer::after(Duration::from_millis(120)).await;
    Ok(())
}
