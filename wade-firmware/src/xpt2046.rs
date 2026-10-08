//! Turning raw XPT2046 resistive-touch readings into the core's touch events
//! (docs/hardware-notes.md#touch).
//!
//! The firmware reads the controller; this module decides what the readings
//! mean. A press is three conversions taken together. While the controller's
//! interrupt line shows a touch, a press with enough pressure moves the touch
//! to the median of its readings; when the line clears, the touch ends where
//! it last was, ignoring the noise that follows a release.

use embedded_graphics::geometry::Point;
use wade_core::layout::SCREEN_SIZE;
use wade_core::{Touch, TouchPhase};

/// One conversion of each channel, 12 bits, in the controller's own axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    pub x: u16,
    pub y: u16,
    /// Pressure: near 0 when nothing presses, higher the firmer the press.
    pub z1: u16,
}

/// Below this pressure a reading is noise, not a press. Real presses on the CYD
/// read from about 160 to 1,300 depending on where they land.
pub const MIN_PRESSURE: u16 = 60;

/// How raw readings map onto the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calibration {
    /// The controller's X channel runs along the screen's y axis.
    pub swap_axes: bool,
    /// Raw readings at the screen's left and right edges.
    pub x: (i32, i32),
    /// Raw readings at the screen's top and bottom edges.
    pub y: (i32, i32),
}

impl Calibration {
    /// The CYD's panel, measured during its bring-up.
    pub const CYD: Calibration = Calibration {
        swap_axes: true,
        x: (166, 3744),
        y: (272, 3908),
    };

    /// The screen point for a raw reading, clamped to the screen.
    #[must_use]
    pub fn to_screen(&self, raw_x: u16, raw_y: u16) -> Point {
        let (along_x, along_y) = if self.swap_axes {
            (raw_y, raw_x)
        } else {
            (raw_x, raw_y)
        };
        let width = SCREEN_SIZE.width.cast_signed();
        let height = SCREEN_SIZE.height.cast_signed();
        Point::new(
            scale(along_x, self.x, width).clamp(0, width - 1),
            scale(along_y, self.y, height).clamp(0, height - 1),
        )
    }
}

/// Map `raw` from the range `edges` onto 0..`size`.
fn scale(raw: u16, (start, end): (i32, i32), size: i32) -> i32 {
    if start == end {
        return 0;
    }
    (i32::from(raw) - start) * size / (end - start)
}

fn median([a, b, c]: [u16; 3]) -> u16 {
    a.max(b).min(a.min(b).max(c))
}

/// Follows one finger or stylus from press to release.
#[derive(Clone, Debug)]
pub struct Tracker {
    calibration: Calibration,
    /// Where the current touch is, while there is one.
    at: Option<Point>,
}

impl Tracker {
    #[must_use]
    pub const fn new(calibration: Calibration) -> Self {
        Tracker {
            calibration,
            at: None,
        }
    }

    /// Three readings taken while the interrupt line shows a touch. Returns the
    /// touch's start or movement, or nothing if the readings are too light to
    /// trust or the touch has not moved.
    pub fn pressed(&mut self, samples: [Sample; 3]) -> Option<Touch> {
        if samples.iter().any(|s| s.z1 < MIN_PRESSURE) {
            return None;
        }
        let point = self
            .calibration
            .to_screen(median(samples.map(|s| s.x)), median(samples.map(|s| s.y)));
        let phase = match self.at.replace(point) {
            None => TouchPhase::Down,
            Some(last) if last == point => return None,
            Some(_) => TouchPhase::Move,
        };
        Some(Touch { phase, point })
    }

    /// The interrupt line no longer shows a touch. Returns the end of the
    /// current touch, if there was one.
    pub fn released(&mut self) -> Option<Touch> {
        self.at.take().map(|point| Touch {
            phase: TouchPhase::Up,
            point,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(x: u16, y: u16) -> [Sample; 3] {
        [Sample { x, y, z1: 300 }; 3]
    }

    #[test]
    fn the_cyd_calibration_maps_its_crosshairs() {
        // Raw medians at the bring-up's crosshairs, which sat at x 20 and 300, y 20 and 220.
        let c = Calibration::CYD;
        let near = |p: Point, x: i32, y: i32| (p.x - x).abs() <= 2 && (p.y - y).abs() <= 2;
        assert!(near(c.to_screen(575, 390), 20, 20), "top-left");
        assert!(near(c.to_screen(578, 3520), 300, 20), "top-right");
        assert!(near(c.to_screen(3605, 3525), 300, 220), "bottom-right");
        assert!(near(c.to_screen(3605, 385), 20, 220), "bottom-left");
    }

    #[test]
    fn readings_past_the_edges_stay_on_screen() {
        let c = Calibration::CYD;
        assert_eq!(c.to_screen(0, 0), Point::new(0, 0));
        assert_eq!(c.to_screen(4095, 4095), Point::new(319, 239));
    }

    #[test]
    fn a_degenerate_calibration_does_not_divide_by_zero() {
        let c = Calibration {
            swap_axes: false,
            x: (100, 100),
            y: (100, 100),
        };
        assert_eq!(c.to_screen(500, 500), Point::zero());
    }

    #[test]
    fn a_touch_goes_down_moves_and_comes_up_where_it_last_was() {
        let mut t = Tracker::new(Calibration::CYD);
        let down = t.pressed(press(575, 390)).unwrap();
        assert_eq!(down.phase, TouchPhase::Down);
        assert_eq!(t.pressed(press(575, 390)), None, "no movement, no event");
        let moved = t.pressed(press(1000, 1000)).unwrap();
        assert_eq!(moved.phase, TouchPhase::Move);
        assert_eq!(
            t.released(),
            Some(Touch {
                phase: TouchPhase::Up,
                point: moved.point
            })
        );
        assert_eq!(t.released(), None, "one Up per touch");
    }

    #[test]
    fn light_readings_are_ignored() {
        let mut t = Tracker::new(Calibration::CYD);
        let mut light = press(575, 390);
        light[1].z1 = MIN_PRESSURE - 1;
        assert_eq!(t.pressed(light), None);
        assert_eq!(
            t.released(),
            None,
            "a touch that never registered never ends"
        );
    }

    #[test]
    fn the_median_discards_one_wild_reading() {
        let mut t = Tracker::new(Calibration {
            swap_axes: false,
            x: (0, 3200),
            y: (0, 2400),
        });
        let mut samples = press(1600, 1200);
        samples[2].x = 4095;
        assert_eq!(
            t.pressed(samples),
            Some(Touch {
                phase: TouchPhase::Down,
                point: Point::new(160, 120)
            })
        );
    }

    #[test]
    fn median_of_three() {
        for (v, m) in [
            ([1, 2, 3], 2),
            ([3, 1, 2], 2),
            ([2, 3, 1], 2),
            ([5, 5, 1], 5),
            ([9, 0, 9], 9),
        ] {
            assert_eq!(median(v), m, "{v:?}");
        }
    }
}
