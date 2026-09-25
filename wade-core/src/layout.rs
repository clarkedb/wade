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

/// The back button, top-left on every app screen.
pub const BACK: Rectangle = Rectangle::new(Point::new(0, 0), NAV_SIZE);
/// The apps button, bottom-right on the Buddy screen.
pub const APPS: Rectangle = Rectangle::new(Point::new(272, 192), NAV_SIZE);

/// The four corners, reserved for navigation on every screen: back top-left,
/// apps bottom-right (docs/ui.md#layout). Nothing else takes touches there.
pub const NAV_CORNERS: [Rectangle; 4] = [
    BACK,
    TITLE,
    Rectangle::new(Point::new(0, 192), NAV_SIZE),
    APPS,
];
const NAV_SIZE: Size = Size::new(48, 48);

/// Where an app screen's title icon sits: the top-right corner, which takes
/// no touches.
pub const TITLE: Rectangle = Rectangle::new(Point::new(272, 0), NAV_SIZE);

/// Something on screen that a tap can land on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Wade,
    Apps,
    Back,
}

/// The target under `point` on the Buddy screen, if any.
#[must_use]
pub fn hit_buddy(point: Point) -> Option<Target> {
    if APPS.contains(point) {
        Some(Target::Apps)
    } else {
        WADE_FACE.contains(point).then_some(Target::Wade)
    }
}

/// The target under `point` on the Timer screen, if any.
#[must_use]
pub fn hit_timer(point: Point) -> Option<Target> {
    BACK.contains(point).then_some(Target::Back)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Screen;

    /// Every hit area on every screen, with a target that uses it, and the
    /// navigation corner it owns, if any.
    const HIT_AREAS: [(Screen, Target, Rectangle, Option<Rectangle>); 3] = [
        (Screen::Buddy, Target::Wade, WADE_FACE, None),
        (Screen::Buddy, Target::Apps, APPS, Some(APPS)),
        (Screen::Timer, Target::Back, BACK, Some(BACK)),
    ];

    #[test]
    fn center_hits_wade_and_corner_does_not() {
        assert_eq!(hit_buddy(WADE_CENTER), Some(Target::Wade));
        assert_eq!(hit_buddy(Point::new(2, 2)), None);
    }

    #[test]
    fn each_navigation_button_hits_only_on_its_screen() {
        assert_eq!(hit_buddy(APPS.center()), Some(Target::Apps));
        assert_eq!(hit_timer(APPS.center()), None);
        assert_eq!(hit_timer(BACK.center()), Some(Target::Back));
        assert_eq!(hit_buddy(BACK.center()), None);
    }

    #[test]
    fn touch_targets_are_large_enough_and_clear_of_the_corners() {
        for (_, target, area, owned) in HIT_AREAS {
            assert!(
                area.size.width >= 48 && area.size.height >= 48,
                "{target:?} is {} px",
                area.size
            );
            for corner in NAV_CORNERS {
                assert!(
                    owned == Some(corner) || area.intersection(&corner).is_zero_sized(),
                    "{target:?} overlaps the corner at {corner:?}"
                );
            }
        }
    }

    #[test]
    fn touch_targets_on_one_screen_do_not_overlap() {
        for (i, &(screen, a, area, _)) in HIT_AREAS.iter().enumerate() {
            for &(other_screen, b, other, _) in &HIT_AREAS[i + 1..] {
                assert!(
                    screen != other_screen || area.intersection(&other).is_zero_sized(),
                    "{a:?} overlaps {b:?}"
                );
            }
        }
    }
}
