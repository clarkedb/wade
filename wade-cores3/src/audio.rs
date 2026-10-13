//! The chime through the AW88298 amplifier over I²S (docs/hardware-notes.md#cores3-lite).
//!
//! The chime is synthesized once at startup into a DMA buffer, as 16-bit stereo.
//! The amplifier is powered only while it plays, at the volume the app task
//! last set.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use esp_hal::Async;
use esp_hal::dma::DmaTxBuf;
use esp_hal::i2s::master::I2sTx;
use log::warn;
use wade_core::settings::Volume;
use wade_core::sound::CHIME;
use wade_firmware::bell;

use crate::Bus;

pub const SAMPLE_RATE: u32 = 16_000;
/// The chime as 16-bit stereo frames.
pub const CHIME_BYTES: usize = 12_000 * 4;
/// Peak sample level as a share of full scale. With the amplifier's volume
/// register, sets how loud the chime is.
const LEVEL: f32 = 0.5;

const AW88298: u8 = 0x36;
const SYSCTRL: u8 = 0x04;
/// I²S on, amplifier and chip powered up.
const SYSCTRL_ON: u16 = 0x4040;
/// I²S off; the chip stays configured.
const SYSCTRL_OFF: u16 = 0x4000;
/// I²S format with 32 bit clocks a frame, and the rate index for 16 kHz.
const I2SCTRL_16K: u16 = 0x14C3;
const VOLCTRL: u8 = 0x0C;

/// A chime request from the app task.
#[derive(Clone, Copy, Debug)]
pub struct Chime;

pub const CHIME_CAPACITY: usize = 2;

async fn write(bus: &mut Bus, reg: u8, value: u16) {
    let [hi, lo] = value.to_be_bytes();
    if let Err(e) = embedded_hal_async::i2c::I2c::write(bus, AW88298, &[reg, hi, lo]).await {
        warn!("amplifier register {reg:#04x} not set: {e:?}");
    }
}

/// The volume register at `volume`. Each step of its top four bits is audibly
/// quieter, and full volume (`0x0064`) is too loud for a desk.
const fn volume_register(volume: Volume) -> Option<u16> {
    match volume {
        // Nothing plays, so the amplifier keeps its last volume.
        Volume::Off => None,
        Volume::Soft => Some(0x3064),
        Volume::Medium => Some(0x2064),
        Volume::Loud => Some(0x1064),
    }
}

/// Set how loud the next chime plays.
pub async fn set_volume(bus: &mut Bus, volume: Volume) {
    if let Some(value) = volume_register(volume) {
        write(bus, VOLCTRL, value).await;
    }
}

/// Fill `buf` with the chime, returning its length in bytes.
fn synthesize(buf: &mut DmaTxBuf) -> usize {
    debug_assert_eq!(bell::total_samples(&CHIME, SAMPLE_RATE) * 4, CHIME_BYTES);
    let frames = buf.as_mut_slice().chunks_exact_mut(4);
    let mut len = 0;
    for (frame, sample) in frames.zip(bell::samples(&CHIME, SAMPLE_RATE, LEVEL)) {
        let [lo, hi] = sample.to_le_bytes();
        frame.copy_from_slice(&[lo, hi, lo, hi]);
        len += 4;
    }
    len
}

#[embassy_executor::task]
pub async fn run(
    mut bus: Bus,
    mut tx: I2sTx<'static, Async>,
    mut buf: DmaTxBuf,
    requests: Receiver<'static, CriticalSectionRawMutex, Chime, CHIME_CAPACITY>,
) {
    let len = synthesize(&mut buf);
    buf.set_length(len);
    write(&mut bus, 0x61, 0x0673).await; // boost off
    write(&mut bus, 0x05, 0x0008).await; // unmuted
    write(&mut bus, 0x06, I2SCTRL_16K).await;
    loop {
        requests.receive().await;
        write(&mut bus, SYSCTRL, SYSCTRL_ON).await;
        match tx.write(buf) {
            Ok(transfer) => {
                let (result, t, b) = transfer.wait_async().await;
                if let Err(e) = result {
                    warn!("chime DMA failed: {e:?}");
                }
                (tx, buf) = (t, b);
            }
            Err((e, t, b)) => {
                warn!("chime not played: {e:?}");
                (tx, buf) = (t, b);
            }
        }
        write(&mut bus, SYSCTRL, SYSCTRL_OFF).await;
    }
}
