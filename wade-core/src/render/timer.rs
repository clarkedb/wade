//! The Timer screen (docs/ui.md#timer).

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StrokeAlignment},
};

use crate::layout::{self, Target};
use crate::render::icons::{self, Icon};
use crate::render::{digits, palette, widgets};
use crate::timer::{RowButton, TimerButton};
use crate::view::TimerView;

/// Corner radius and outline width of the row's buttons.
const BUTTON_RADIUS: u32 = 12;
const BUTTON_LINE: u32 = 3;

pub fn draw<D>(view: TimerView, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    widgets::back_button(view.pressed == Some(Target::Back), target)?;
    icons::draw(Icon::Stopwatch, layout::TITLE, palette::EYE, target)?;
    digits::time(view.digits, target)?;
    for (&slot, button) in layout::TIMER_ROW.iter().zip(view.row) {
        if let Some(button) = button {
            let pressed = view.pressed == Some(Target::Timer(button.button));
            row_button(slot, button, pressed, target)?;
        }
    }
    Ok(())
}

/// A button in the row: outlined, filled while pressed, and dimmed while it
/// ignores taps.
fn row_button<D>(
    slot: Rectangle,
    button: RowButton,
    pressed: bool,
    target: &mut D,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let color = if button.enabled {
        palette::EYE
    } else {
        palette::DIM
    };
    let mut style = PrimitiveStyleBuilder::new()
        .stroke_color(color)
        .stroke_width(BUTTON_LINE)
        .stroke_alignment(StrokeAlignment::Inside);
    if pressed {
        style = style.fill_color(color);
    }
    RoundedRectangle::with_equal_corners(slot, Size::new_equal(BUTTON_RADIUS))
        .into_styled(style.build())
        .draw(target)?;

    let color = if pressed { palette::BACKGROUND } else { color };
    icons::draw(icon(button.button), slot, color, target)
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
