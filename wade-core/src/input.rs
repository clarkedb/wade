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
    #[must_use]
    pub const fn new() -> Self {
        TouchTracker { pressed: None }
    }

    /// Feed one raw sample. `hit` maps a point to the target under it on the
    /// current screen. Returns the target when a tap completes.
    #[must_use = "a returned target is a completed tap"]
    pub fn handle(
        &mut self,
        touch: Touch,
        hit: impl Fn(Point) -> Option<Target>,
    ) -> Option<Target> {
        match touch.phase {
            TouchPhase::Down => {
                self.pressed = hit(touch.point);
                None
            }
            TouchPhase::Move => {
                self.pressed = self
                    .pressed
                    .filter(|&target| hit(touch.point) == Some(target));
                None
            }
            TouchPhase::Up => self
                .pressed
                .take()
                .filter(|&target| hit(touch.point) == Some(target)),
        }
    }

    /// The target currently pressed, for drawing a pressed style.
    #[must_use]
    pub const fn pressed(&self) -> Option<Target> {
        self.pressed
    }

    /// Ignore the rest of the current touch, for example after a screen change.
    pub fn cancel(&mut self) {
        self.pressed = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSIDE: Point = Point::new(10, 10);
    const ALSO_INSIDE: Point = Point::new(20, 10);
    const OUTSIDE: Point = Point::new(200, 10);

    fn hit(point: Point) -> Option<Target> {
        (point.x < 100).then_some(Target::Wade)
    }

    fn sample(tracker: &mut TouchTracker, phase: TouchPhase, point: Point) -> Option<Target> {
        tracker.handle(Touch { phase, point }, hit)
    }

    #[test]
    fn a_tap_fires_on_up_over_its_target() {
        let mut t = TouchTracker::new();
        assert_eq!(sample(&mut t, TouchPhase::Down, INSIDE), None);
        assert_eq!(t.pressed(), Some(Target::Wade));
        assert_eq!(sample(&mut t, TouchPhase::Move, ALSO_INSIDE), None);
        assert_eq!(
            sample(&mut t, TouchPhase::Up, ALSO_INSIDE),
            Some(Target::Wade)
        );
        assert_eq!(t.pressed(), None);
    }

    #[test]
    fn moving_off_the_target_cancels_the_tap_even_if_it_returns() {
        let mut t = TouchTracker::new();
        let _ = sample(&mut t, TouchPhase::Down, INSIDE);
        let _ = sample(&mut t, TouchPhase::Move, OUTSIDE);
        assert_eq!(t.pressed(), None);
        let _ = sample(&mut t, TouchPhase::Move, INSIDE);
        assert_eq!(sample(&mut t, TouchPhase::Up, INSIDE), None);
    }

    #[test]
    fn up_away_from_the_target_is_not_a_tap() {
        let mut t = TouchTracker::new();
        let _ = sample(&mut t, TouchPhase::Down, INSIDE);
        assert_eq!(sample(&mut t, TouchPhase::Up, OUTSIDE), None);
    }

    #[test]
    fn up_without_down_and_a_cancelled_press_do_nothing() {
        let mut t = TouchTracker::new();
        assert_eq!(sample(&mut t, TouchPhase::Up, INSIDE), None);
        let _ = sample(&mut t, TouchPhase::Down, INSIDE);
        t.cancel();
        assert_eq!(sample(&mut t, TouchPhase::Up, INSIDE), None);
    }
}
