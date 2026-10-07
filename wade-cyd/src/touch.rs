//! The CYD's XPT2046 resistive touch controller (docs/hardware-notes.md#touch).
//!
//! Waits for the controller's interrupt line, then samples about 100 times a
//! second until it clears. `wade_firmware::xpt2046` turns the samples into the
//! core's touch events, which go to the app task stamped with the time.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};
use esp_hal::Blocking;
use esp_hal::gpio::{Input, Output};
use esp_hal::spi::master::Spi;
use log::warn;
use wade_core::{Event, EventKind, Touch};
use wade_firmware::xpt2046::{Calibration, Sample, Tracker};

use crate::now;

/// Control bytes: start a 12-bit conversion of one channel, leaving the
/// reference and ADC on so the next conversion can follow at once.
const READ_Z1: u8 = 0xB1;
const READ_X: u8 = 0xD1;
const READ_Y: u8 = 0x91;
/// A last conversion that powers down and re-enables the interrupt line.
const POWER_DOWN: u8 = 0xD0;

const SAMPLE_INTERVAL: Duration = Duration::from_millis(10);

pub struct TouchPanel<'d> {
    pub spi: Spi<'d, Blocking>,
    pub cs: Output<'d>,
    /// Low while the panel is touched.
    pub irq: Input<'d>,
}

impl TouchPanel<'_> {
    fn read(&mut self, control: u8) -> u16 {
        let mut buf = [control, 0, 0];
        self.cs.set_low();
        let result = self.spi.transfer(&mut buf);
        self.cs.set_high();
        if let Err(e) = result {
            warn!("touch read failed: {e:?}");
            return 0;
        }
        (u16::from_be_bytes([buf[1], buf[2]]) >> 3) & 0x0FFF
    }

    fn sample(&mut self) -> Sample {
        let z1 = self.read(READ_Z1);
        let x = self.read(READ_X);
        let y = self.read(READ_Y);
        let _ = self.read(POWER_DOWN);
        Sample { x, y, z1 }
    }
}

#[embassy_executor::task]
pub async fn run(
    mut panel: TouchPanel<'static>,
    events: Sender<'static, CriticalSectionRawMutex, Event, { crate::EVENT_CAPACITY }>,
) {
    let mut tracker = Tracker::new(Calibration::CYD);
    let send = |touch: Touch| {
        let event = Event {
            at: now(),
            kind: EventKind::Touch(touch),
        };
        if events.try_send(event).is_err() {
            warn!("event queue full; dropped {touch:?}");
        }
    };
    loop {
        panel.irq.wait_for_low().await;
        while panel.irq.is_low() {
            let samples = [panel.sample(), panel.sample(), panel.sample()];
            // A release during sampling leaves the last samples unreliable.
            if panel.irq.is_low()
                && let Some(touch) = tracker.pressed(samples)
            {
                send(touch);
            }
            Timer::after(SAMPLE_INTERVAL).await;
        }
        if let Some(touch) = tracker.released() {
            send(touch);
        }
    }
}
