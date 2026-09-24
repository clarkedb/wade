//! What is on screen, as plain data. The core produces it; `render::draw` turns it into pixels.

use crate::character::{Expression, Pose};

#[derive(Clone, Debug, PartialEq)]
pub enum View {
    Buddy(BuddyView),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuddyView {
    /// The discrete expression, for behavior tests and the state hash.
    pub expression: Expression,
    /// True while a blink is in progress.
    pub blinking: bool,
    /// The continuous pose that drawing reads. Behavior tests do not compare it.
    pub pose: Pose,
}
