//! Screen geometry shared by drawing and hit-testing, so what you see is what you can tap.

use embedded_graphics::{
    geometry::{Point, Size},
    primitives::{Circle, Rectangle},
};

/// Logical screen size: 320×240, landscape, origin top-left.
pub const SCREEN_SIZE: Size = Size::new(320, 240);
pub const SCREEN: Rectangle = Rectangle::new(Point::zero(), SCREEN_SIZE);

/// Centre of Wade's head on the Buddy screen.
pub const WADE_CENTER: Point = Point::new(160, 120);
/// Diameter of Wade's head. The face must be at least 160 px tall (docs/character.md#appearance).
pub const WADE_HEAD_DIAMETER: u32 = 180;
/// Wade's head, which is also his hit area.
pub const WADE_HEAD: Circle = Circle::with_center(WADE_CENTER, WADE_HEAD_DIAMETER);

/// Something on screen that a tap can land on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Wade,
}

/// The target under `point` on the Buddy screen, if any.
pub fn hit_buddy(point: Point) -> Option<Target> {
    use embedded_graphics::primitives::ContainsPoint;
    WADE_HEAD.contains(point).then_some(Target::Wade)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centre_hits_wade_and_corner_does_not() {
        assert_eq!(hit_buddy(WADE_CENTER), Some(Target::Wade));
        assert_eq!(hit_buddy(Point::new(2, 2)), None);
    }
}
