//! Turning FT6336U capacitive-touch reports into the core's touch events
//! (docs/hardware-notes.md#cores3-lite).
//!
//! The controller reports in screen coordinates, so there is no calibration or
//! filtering: a report with a touch starts or moves it, and one without ends it.

use embedded_graphics::geometry::Point;
use wade_core::layout::SCREEN_SIZE;
use wade_core::{Touch, TouchPhase};

/// The register a report starts at, and its length: the touch count, then the
/// first touch's X and Y.
pub const REPORT_START: u8 = 0x02;
pub const REPORT_LEN: usize = 5;

/// The first touch in a report, clamped to the screen, or `None` if nothing
/// touches. Counts above two are the controller's way of saying "no data".
#[must_use]
pub fn point(report: [u8; REPORT_LEN]) -> Option<Point> {
    let count = report[0] & 0x0F;
    if !(1..=2).contains(&count) {
        return None;
    }
    let x = i32::from(u16::from(report[1] & 0x0F) << 8 | u16::from(report[2]));
    let y = i32::from(u16::from(report[3] & 0x0F) << 8 | u16::from(report[4]));
    Some(Point::new(
        x.min(SCREEN_SIZE.width.cast_signed() - 1),
        y.min(SCREEN_SIZE.height.cast_signed() - 1),
    ))
}

/// Follows one finger from press to release.
#[derive(Clone, Debug, Default)]
pub struct Tracker {
    /// Where the current touch is, while there is one.
    at: Option<Point>,
}

impl Tracker {
    #[must_use]
    pub const fn new() -> Self {
        Tracker { at: None }
    }

    /// A report read from the controller. Returns the touch's start, movement,
    /// or end, or nothing if nothing changed.
    pub fn report(&mut self, report: [u8; REPORT_LEN]) -> Option<Touch> {
        let Some(point) = point(report) else {
            return self.released();
        };
        let phase = match self.at.replace(point) {
            None => TouchPhase::Down,
            Some(last) if last == point => return None,
            Some(_) => TouchPhase::Move,
        };
        Some(Touch { phase, point })
    }

    /// The touch can no longer be read. Returns its end, if there was one.
    pub fn released(&mut self) -> Option<Touch> {
        self.at.take().map(|point| Touch {
            phase: TouchPhase::Up,
            point,
        })
    }

    /// Whether a touch is in progress.
    #[must_use]
    pub const fn touching(&self) -> bool {
        self.at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: u16, y: u16) -> [u8; REPORT_LEN] {
        let [xh, xl] = x.to_be_bytes();
        let [yh, yl] = y.to_be_bytes();
        // Event flags in X's top bits and the touch ID in Y's are not position.
        [1, 0x80 | xh, xl, 0x10 | yh, yl]
    }

    const NONE: [u8; REPORT_LEN] = [0, 0xFF, 0xFF, 0xFF, 0xFF];

    #[test]
    fn a_report_reads_as_screen_coordinates() {
        assert_eq!(point(at(173, 147)), Some(Point::new(173, 147)));
        assert_eq!(point(at(319, 239)), Some(Point::new(319, 239)));
    }

    #[test]
    fn reports_past_the_edges_stay_on_screen() {
        assert_eq!(point(at(400, 300)), Some(Point::new(319, 239)));
    }

    #[test]
    fn no_touch_and_invalid_counts_are_nothing() {
        assert_eq!(point(NONE), None);
        let mut bad = at(10, 10);
        bad[0] = 0x0F;
        assert_eq!(point(bad), None);
    }

    #[test]
    fn a_touch_goes_down_moves_and_comes_up_where_it_last_was() {
        let mut t = Tracker::new();
        assert_eq!(t.report(NONE), None, "no touch, no event");
        let down = t.report(at(20, 60)).unwrap();
        assert_eq!(down.phase, TouchPhase::Down);
        assert!(t.touching());
        assert_eq!(t.report(at(20, 60)), None, "no movement, no event");
        let moved = t.report(at(40, 60)).unwrap();
        assert_eq!(moved.phase, TouchPhase::Move);
        assert_eq!(
            t.report(NONE),
            Some(Touch {
                phase: TouchPhase::Up,
                point: Point::new(40, 60)
            })
        );
        assert!(!t.touching());
        assert_eq!(t.released(), None, "one Up per touch");
    }
}
