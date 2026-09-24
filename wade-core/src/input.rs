//! `TouchTracker`: turns raw touch samples into taps (docs/ui.md#touch-handling).
//!
//! A tap targets the element under the `Down` point and fires on `Up` if the `Up`
//! point is still inside that element. Moving outside cancels it. A screen change
//! cancels any tap in progress.

use embedded_graphics::geometry::Point;

use crate::event::{Touch, TouchPhase};
use crate::layout::Target;

#[derive(Clone, Debug, Default)]
pub struct TouchTracker {
    /// The target under the current `Down`, while it is still a candidate tap.
    pressed: Option<Target>,
}

impl TouchTracker {
    pub const fn new() -> Self {
        TouchTracker { pressed: None }
    }

    /// Feed one raw sample. `hit` maps a point to the target under it on the
    /// current screen. Returns the target when a tap completes.
    pub fn handle(
        &mut self,
        touch: Touch,
        hit: impl Fn(Point) -> Option<Target>,
    ) -> Option<Target> {
        // TODO(M1): track Down → Move → Up and fire only when Up lands on the Down target.
        let _ = (touch.phase == TouchPhase::Down, hit(touch.point));
        None
    }

    /// The target currently pressed, for drawing a pressed style.
    pub const fn pressed(&self) -> Option<Target> {
        self.pressed
    }

    /// Ignore the rest of the current touch, for example after a screen change.
    pub fn cancel(&mut self) {
        self.pressed = None;
    }
}
