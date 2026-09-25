//! What is on screen, as plain data. The core produces it; `render::draw` turns it into pixels.

use crate::character::{Expression, Pose};
use crate::layout::Target;
use crate::timer::{Digits, RowButton, TimerPhase};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
    Buddy(BuddyView),
    Timer(TimerView),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BuddyView {
    /// The discrete expression, for behavior tests and the state hash.
    pub expression: Expression,
    /// True while Wade is asleep, which the expression alone does not show.
    pub asleep: bool,
    /// True while a blink is in progress.
    pub blinking: bool,
    /// The continuous pose that drawing reads. Behavior tests do not compare it.
    pub pose: Pose,
    pub eye_style: EyeStyle,
    /// True while a touch that began on the apps button stays on it.
    pub apps_pressed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerView {
    pub phase: TimerPhase,
    /// The time shown: 00:00 once Done.
    pub digits: Digits,
    /// The row's left, center, and right buttons. `None` leaves a slot empty.
    pub row: [Option<RowButton>; 3],
    /// The button under a touch in progress, drawn pressed.
    pub pressed: Option<Target>,
}

/// How Wade's eyes are drawn: a setting, not part of his behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum EyeStyle {
    #[default]
    Pupils,
    Plain,
}

impl EyeStyle {
    /// The other style.
    #[must_use]
    pub const fn toggled(self) -> EyeStyle {
        match self {
            EyeStyle::Pupils => EyeStyle::Plain,
            EyeStyle::Plain => EyeStyle::Pupils,
        }
    }
}
