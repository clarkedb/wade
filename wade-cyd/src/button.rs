//! The CYD's active-low BOOT button on GPIO0, used for sleep and wake.

use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Event as GpioEvent, Input, Level};
use log::warn;
use wade_core::Event;

use crate::now;

const DEBOUNCE: Duration = Duration::from_millis(30);

/// Sends one power-button event per press, with no repeat while held.
#[embassy_executor::task]
pub async fn run(
    mut button: Input<'static>,
    events: Sender<'static, CriticalSectionRawMutex, Event, { crate::EVENT_CAPACITY }>,
) {
    // A button held during startup must be released before it counts as input.
    wait_for_stable_level(&mut button, Level::High).await;
    loop {
        wait_for_stable_level(&mut button, Level::Low).await;
        if events.try_send(Event::power_button(now())).is_err() {
            warn!("event queue full; dropped a power button press");
        }
        wait_for_stable_level(&mut button, Level::High).await;
    }
}

/// Restarts the debounce interval whenever the input leaves the target level.
async fn wait_for_stable_level(button: &mut Input<'_>, level: Level) {
    let (target, opposite) = match level {
        Level::Low => (GpioEvent::LowLevel, GpioEvent::HighLevel),
        Level::High => (GpioEvent::HighLevel, GpioEvent::LowLevel),
    };
    loop {
        button.wait_for(target).await;
        if matches!(
            select(button.wait_for(opposite), Timer::after(DEBOUNCE)).await,
            Either::Second(())
        ) && button.level() == level
        {
            return;
        }
    }
}
