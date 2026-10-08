#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

//! Step 8: Wade's chime through the AW88298 amplifier over I²S, then a 440 Hz tone at three
//! amplifier volumes. Repeats every 10 s.

use core::f32::consts::TAU;

use cores3_bringup::power::{self, AW9523B};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::Blocking;
use esp_hal::clock::CpuClock;
use esp_hal::dma::DmaTxBuf;
use esp_hal::dma_tx_buffer;
use esp_hal::i2c::master::{self, I2c};
use esp_hal::i2s::master::{Channels, DataFormat, I2s, I2sTx, TdmConfig};
use esp_hal::time::{Instant, Rate};
use esp_hal::timer::timg::TimerGroup;
use log::info;
use wade_core::sound::CHIME;

esp_bootloader_esp_idf::esp_app_desc!();

const AW88298: u8 = 0x36;
const SAMPLE_RATE: u32 = 16_000;
const SAMPLES: usize = 12_000; // the chime's 750 ms
const BYTES: usize = SAMPLES * 4; // 16-bit stereo
const OVERTONE: f32 = 0.2;
const ATTACK: f32 = 0.004;
const RELEASE: f32 = 0.006;
/// Peak sample level, as a fraction of full scale.
const LEVEL: f32 = 0.5;

fn aw_write(i2c: &mut I2c<'_, Blocking>, reg: u8, value: u16) {
    let [hi, lo] = value.to_be_bytes();
    i2c.write(AW88298, &[reg, hi, lo]).unwrap();
}

fn aw_read(i2c: &mut I2c<'_, Blocking>, reg: u8) -> u16 {
    let mut b = [0u8; 2];
    i2c.write_read(AW88298, &[reg], &mut b).unwrap();
    u16::from_be_bytes(b)
}

/// Write one 16-bit sample to both channels at frame `i`.
fn put(out: &mut [u8], i: usize, s: f32) {
    let [lo, hi] = ((s * LEVEL * 32767.0) as i16).to_le_bytes();
    out[i * 4..i * 4 + 4].copy_from_slice(&[lo, hi, lo, hi]);
}

/// The desktop's bell synthesis. Returns the bytes written.
fn chime(out: &mut [u8]) -> usize {
    let mut n = 0;
    for tone in CHIME {
        let length = tone.duration.as_millis() as f32 / 1000.0;
        let decay = length / 4.0;
        let hz = f32::from(tone.hz);
        let count = (length * SAMPLE_RATE as f32) as usize;
        for i in 0..count {
            let t = i as f32 / SAMPLE_RATE as f32;
            let envelope =
                (t / ATTACK).min(1.0) * ((length - t) / RELEASE).min(1.0) * libm::expf(-t / decay);
            let wave = libm::sinf(TAU * hz * t) + OVERTONE * libm::sinf(2.0 * TAU * hz * t);
            put(out, n, envelope * wave / (1.0 + OVERTONE));
            n += 1;
        }
    }
    n * 4
}

/// A 440 Hz sine for 500 ms with short fades. Returns the bytes written.
fn tone(out: &mut [u8]) -> usize {
    let count = SAMPLE_RATE as usize / 2;
    for i in 0..count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let envelope = (t / 0.01).min(1.0) * ((0.5 - t) / 0.01).min(1.0);
        put(out, i, envelope * libm::sinf(TAU * 440.0 * t));
    }
    count * 4
}

fn play<'d>(
    tx: I2sTx<'d, Blocking>,
    mut buf: DmaTxBuf,
    len: usize,
) -> (I2sTx<'d, Blocking>, DmaTxBuf) {
    buf.set_length(len);
    let start = Instant::now();
    let transfer = match tx.write(buf) {
        Ok(t) => t,
        Err((e, _, _)) => panic!("i2s write: {e:?}"),
    };
    let (result, tx, buf) = transfer.wait();
    info!(
        "  played {len} bytes in {} ms: {result:?}",
        start.elapsed().as_millis()
    );
    (tx, buf)
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
        "AW9523B port 0 out {:08b}",
        power::read(&mut i2c, AW9523B, 0x02).unwrap()
    );
    info!("AW88298 id {:#06x}", aw_read(&mut i2c, 0x00));

    let i2s = I2s::new(
        peripherals.I2S1,
        peripherals.DMA_CH1,
        TdmConfig::new_tdm_philips()
            .with_sample_rate(Rate::from_hz(SAMPLE_RATE))
            .with_data_format(DataFormat::Data16Channel16)
            .with_channels(Channels::STEREO),
    )
    .unwrap();
    let mut tx = i2s
        .i2s_tx
        .with_bclk(peripherals.GPIO34)
        .with_ws(peripherals.GPIO33)
        .with_dout(peripherals.GPIO13)
        .build();

    // From M5Unified: boost off, I2S on and amplifier powered, unmuted, 16 kHz with 32 BCK
    // per frame.
    aw_write(&mut i2c, 0x61, 0x0673);
    aw_write(&mut i2c, 0x04, 0x4040);
    aw_write(&mut i2c, 0x05, 0x0008);
    aw_write(&mut i2c, 0x06, 0x14C3);
    for reg in [0x01, 0x04, 0x05, 0x06, 0x0C] {
        info!("AW88298 reg {reg:#04x} = {:#06x}", aw_read(&mut i2c, reg));
    }

    let mut buf = dma_tx_buffer!(BYTES).unwrap();
    loop {
        aw_write(&mut i2c, 0x0C, 0x0064);
        info!("chime");
        let len = chime(buf.as_mut_slice());
        (tx, buf) = play(tx, buf, len);
        Timer::after(Duration::from_secs(1)).await;
        for vol in [0x0064u16, 0x1064, 0x2064] {
            aw_write(&mut i2c, 0x0C, vol);
            info!("440 Hz, volume register {vol:#06x}");
            let len = tone(buf.as_mut_slice());
            (tx, buf) = play(tx, buf, len);
            Timer::after(Duration::from_millis(500)).await;
        }
        info!("AW88298 status {:#06x}", aw_read(&mut i2c, 0x01));
        Timer::after(Duration::from_secs(10)).await;
    }
}
