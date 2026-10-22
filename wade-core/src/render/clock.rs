//! The countdown Wade keeps on the Buddy screen (docs/ui.md#buddy-screen).

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use crate::layout;
use crate::render::{digits, palette};
use crate::timer::TimerPhase;
use crate::view::ClockView;

pub fn draw<D>(clock: ClockView, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    // Bright when it needs attention: finished, or under a finger.
    let color = match clock.phase {
        _ if clock.pressed => palette::EYE,
        TimerPhase::Done => palette::EYE,
        TimerPhase::Paused => palette::DIM,
        TimerPhase::Ready | TimerPhase::Running => palette::CLOCK,
    };
    digits::time(clock.digits, layout::CLOCK, digits::SMALL, color, target)
}
