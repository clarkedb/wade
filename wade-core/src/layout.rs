//! Screen geometry shared by drawing and hit-testing, so what you see is what you can tap.

use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};

use crate::timer::{RowButton, TimerButton};

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

/// The band the timer's digits are centered in, between the title and the buttons.
pub const TIMER_DIGITS: Rectangle = Rectangle::new(Point::new(0, 48), Size::new(320, 88));
/// The timer's row of buttons: left, center, and right, above the bottom corners.
pub const TIMER_ROW: [Rectangle; 3] = [
    Rectangle::new(Point::new(8, 136), Size::new(72, 56)),
    Rectangle::new(Point::new(96, 136), Size::new(128, 56)),
    Rectangle::new(Point::new(240, 136), Size::new(72, 56)),
];

/// The Launcher's grid: four square slots, two by two, centered and clear of
/// the corners.
const GRID: [Rectangle; 4] = [
    Rectangle::new(Point::new(64, 24), TILE_SIZE),
    Rectangle::new(Point::new(168, 24), TILE_SIZE),
    Rectangle::new(Point::new(64, 128), TILE_SIZE),
    Rectangle::new(Point::new(168, 128), TILE_SIZE),
];
const TILE_SIZE: Size = Size::new(88, 88);

/// The Launcher's tiles and their slots. Apps fill the grid from the top
/// left; Settings always takes the bottom right.
pub const TILES: [(Tile, Rectangle); 2] = [(Tile::Timer, GRID[0]), (Tile::Settings, GRID[3])];

/// An app the Launcher opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tile {
    Timer,
    Settings,
}

/// Something on screen that a tap can land on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Wade,
    Apps,
    Back,
    Tile(Tile),
    Timer(TimerButton),
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

/// The target under `point` on the Launcher, if any.
#[must_use]
pub fn hit_launcher(point: Point) -> Option<Target> {
    if BACK.contains(point) {
        return Some(Target::Back);
    }
    TILES
        .iter()
        .find(|(_, area)| area.contains(point))
        .map(|&(tile, _)| Target::Tile(tile))
}

/// The target under `point` on the Settings screen, if any.
#[must_use]
pub fn hit_settings(point: Point) -> Option<Target> {
    BACK.contains(point).then_some(Target::Back)
}

/// The target under `point` on the Timer screen, whose row holds `row`, if
/// any. Disabled buttons take no touches.
#[must_use]
pub fn hit_timer(point: Point, row: [Option<RowButton>; 3]) -> Option<Target> {
    if BACK.contains(point) {
        return Some(Target::Back);
    }
    TIMER_ROW
        .iter()
        .zip(row)
        .find(|(slot, _)| slot.contains(point))
        .and_then(|(_, button)| button)
        .filter(|button| button.enabled)
        .map(|button| Target::Timer(button.button))
}

#[cfg(test)]
mod tests {
    use std::vec;
    use std::vec::Vec;

    use super::*;
    use crate::app::Screen;
    use crate::time::Instant;
    use crate::timer::TimerState;

    /// A hit area on a screen, with a target that uses it, and the
    /// navigation corner it owns, if any.
    type HitArea = (Screen, Target, Rectangle, Option<Rectangle>);

    /// Every hit area on every screen, taken from the layout's own tables so
    /// that a new one is checked too.
    fn hit_areas() -> Vec<HitArea> {
        let mut areas = vec![
            (Screen::Buddy, Target::Wade, WADE_FACE, None),
            (Screen::Buddy, Target::Apps, APPS, Some(APPS)),
        ];
        for screen in [Screen::Launcher, Screen::Timer, Screen::Settings] {
            areas.push((screen, Target::Back, BACK, Some(BACK)));
        }
        areas.extend(
            TILES
                .iter()
                .map(|&(tile, area)| (Screen::Launcher, Target::Tile(tile), area, None)),
        );
        let row = TimerState::new().row();
        areas.extend(TIMER_ROW.iter().zip(row).filter_map(|(&area, button)| {
            button.map(|b| (Screen::Timer, Target::Timer(b.button), area, None))
        }));
        areas
    }

    #[test]
    fn center_hits_wade_and_corner_does_not() {
        assert_eq!(hit_buddy(WADE_CENTER), Some(Target::Wade));
        assert_eq!(hit_buddy(Point::new(2, 2)), None);
    }

    #[test]
    fn each_navigation_button_hits_only_on_its_screen() {
        assert_eq!(hit_buddy(APPS.center()), Some(Target::Apps));
        let timer = |p| hit_timer(p, TimerState::new().row());
        for hit in [hit_launcher, hit_settings, timer] {
            assert_eq!(hit(APPS.center()), None);
            assert_eq!(hit(BACK.center()), Some(Target::Back));
        }
        assert_eq!(hit_buddy(BACK.center()), None);
    }

    #[test]
    fn settings_takes_the_bottom_right_tile() {
        assert_eq!(TILES.last().map(|&(tile, _)| tile), Some(Tile::Settings));
        assert_eq!(TILES.last().map(|&(_, area)| area), GRID.last().copied());
        for (tile, area) in TILES {
            assert_eq!(hit_launcher(area.center()), Some(Target::Tile(tile)));
        }
    }

    #[test]
    fn the_row_hits_only_enabled_buttons_in_filled_slots() {
        let mut timer = TimerState::new();
        let hit = |timer: TimerState, slot: usize| hit_timer(TIMER_ROW[slot].center(), timer.row());
        assert_eq!(hit(timer, 0), Some(Target::Timer(TimerButton::Minus)));
        assert_eq!(hit(timer, 1), Some(Target::Timer(TimerButton::Start)));
        while timer.press(TimerButton::Minus, Instant::from_millis(0)) {}
        assert_eq!(hit(timer, 0), None, "a disabled −1m takes touches");
        assert!(timer.press(TimerButton::Start, Instant::from_millis(0)));
        assert_eq!(hit(timer, 0), None, "an empty slot takes touches");
        assert_eq!(hit(timer, 1), Some(Target::Timer(TimerButton::Pause)));
    }

    #[test]
    fn touch_targets_are_large_enough_and_clear_of_the_corners() {
        for (_, target, area, owned) in hit_areas() {
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
        let areas = hit_areas();
        for (i, &(screen, a, area, _)) in areas.iter().enumerate() {
            for &(other_screen, b, other, _) in &areas[i + 1..] {
                assert!(
                    screen != other_screen || area.intersection(&other).is_zero_sized(),
                    "{a:?} overlaps {b:?}"
                );
            }
        }
    }
}
