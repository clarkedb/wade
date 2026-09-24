//! Screen geometry shared by drawing and hit-testing, so what you see is what you can tap.

use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};

/// Logical screen size: 320×240, landscape, origin top-left.
pub const SCREEN_SIZE: Size = Size::new(320, 240);
pub const SCREEN: Rectangle = Rectangle::new(Point::zero(), SCREEN_SIZE);

/// Center of Wade's face on the Buddy screen.
pub const WADE_CENTER: Point = Point::new(160, 120);
/// Wade's hit area: his eyes and the space around them, clear of the navigation corners.
pub const WADE_FACE: Rectangle = Rectangle::new(Point::new(48, 36), Size::new(224, 168));

/// The four corners, reserved for navigation on every screen: back top-left,
/// apps bottom-right (docs/ui.md#layout). Nothing else takes touches there.
pub const NAV_CORNERS: [Rectangle; 4] = [
    Rectangle::new(Point::new(0, 0), NAV_SIZE),
    Rectangle::new(Point::new(272, 0), NAV_SIZE),
    Rectangle::new(Point::new(0, 192), NAV_SIZE),
    Rectangle::new(Point::new(272, 192), NAV_SIZE),
];
const NAV_SIZE: Size = Size::new(48, 48);

/// Something on screen that a tap can land on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Wade,
}

/// The target under `point` on the Buddy screen, if any.
#[must_use]
pub fn hit_buddy(point: Point) -> Option<Target> {
    WADE_FACE.contains(point).then_some(Target::Wade)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_hits_wade_and_corner_does_not() {
        assert_eq!(hit_buddy(WADE_CENTER), Some(Target::Wade));
        assert_eq!(hit_buddy(Point::new(2, 2)), None);
    }

    #[test]
    fn wade_stays_clear_of_the_navigation_corners() {
        for corner in NAV_CORNERS {
            assert!(
                WADE_FACE.intersection(&corner).is_zero_sized(),
                "Wade's hit area overlaps {corner:?}"
            );
        }
    }
}
