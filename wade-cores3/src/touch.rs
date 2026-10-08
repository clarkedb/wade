//! The CoreS3's FT6336U capacitive touch controller (docs/hardware-notes.md#cores3-lite).
//!
//! Its interrupt reaches GPIO 21 through the AW9523B, which pulls the line low
//! on any change. The task waits for that, then reads the controller about 100
//! times a second while the touch lasts. `wade_firmware::ft6336` turns the
//! reports into the core's touch events, which go to the app task stamped with
//! the time.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c as _;
use esp_hal::gpio::Input;
use log::warn;
use wade_core::{Event, EventKind, Touch};
use wade_firmware::ft6336::{REPORT_LEN, REPORT_START, Tracker};

use crate::{Bus, now, power};

const FT6336: u8 = 0x38;
const SAMPLE_INTERVAL: Duration = Duration::from_millis(10);

#[embassy_executor::task]
pub async fn run(
    mut bus: Bus,
    mut interrupt: Input<'static>,
    events: Sender<'static, CriticalSectionRawMutex, Event, { crate::EVENT_CAPACITY }>,
) {
    let mut tracker = Tracker::new();
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
        interrupt.wait_for_low().await;
        loop {
            // Always sleeps before reading again or re-arming, so a line stuck
            // low cannot keep this task from yielding.
            Timer::after(SAMPLE_INTERVAL).await;
            let held = power::touch_held(&mut bus).await.unwrap_or_else(|e| {
                warn!("touch interrupt unreadable: {e:?}");
                false
            });
            if !held {
                if let Some(touch) = tracker.released() {
                    send(touch);
                }
                break;
            }
            let mut report = [0; REPORT_LEN];
            match bus.write_read(FT6336, &[REPORT_START], &mut report).await {
                Ok(()) => {
                    if let Some(touch) = tracker.report(report) {
                        send(touch);
                    }
                }
                Err(e) => warn!("touch read failed: {e:?}"),
            }
        }
    }
}
