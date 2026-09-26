//! The Timer screen (docs/ui.md#timer).

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use crate::layout::{self, Target};
use crate::render::icons::Icon;
use crate::render::{digits, widgets};
use crate::timer::TimerButton;
use crate::view::TimerView;

pub fn draw<D>(view: TimerView, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    widgets::back_button(view.pressed == Some(Target::Back), target)?;
    widgets::title(Icon::Stopwatch, target)?;
    digits::time(view.digits, target)?;
    for (&slot, button) in layout::TIMER_ROW.iter().zip(view.row) {
        if let Some(button) = button {
            let pressed = view.pressed == Some(Target::Timer(button.button));
            widgets::button(
                slot,
                icon(button.button),
                1,
                button.enabled,
                pressed,
                target,
            )?;
        }
    }
    Ok(())
}

const fn icon(button: TimerButton) -> Icon {
    match button {
        TimerButton::Minus => Icon::Minus,
        TimerButton::Plus => Icon::Plus,
        TimerButton::Start | TimerButton::Resume => Icon::Play,
        TimerButton::Pause => Icon::Pause,
        TimerButton::Reset => Icon::Stop,
        TimerButton::Dismiss => Icon::Check,
    }
}
