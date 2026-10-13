//! Screen geometry shared by drawing and hit-testing, so what you see is what you can tap.

use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};

use crate::settings::SettingsButton;
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
/// The previous page button of a paged grid, pointing up on the right edge.
pub const PREVIOUS_PAGE: Rectangle = Rectangle::new(Point::new(272, 48), NAV_SIZE);
/// The next page button of a paged grid, pointing down on the right edge.
pub const NEXT_PAGE: Rectangle = Rectangle::new(Point::new(272, 144), NAV_SIZE);

/// The four corners, reserved for navigation on every screen: back top-left,
/// apps bottom-right on Buddy; the other corners take no touches (docs/ui.md#layout).
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

/// The grid of the Launcher and Settings screens: four square slots, two by
/// two, centered and clear of the corners. A screen with more tiles pages
/// through them, four at a time.
const GRID: [Rectangle; 4] = [
    Rectangle::new(Point::new(64, 24), TILE_SIZE),
    Rectangle::new(Point::new(168, 24), TILE_SIZE),
    Rectangle::new(Point::new(64, 128), TILE_SIZE),
    Rectangle::new(Point::new(168, 128), TILE_SIZE),
];
const TILE_SIZE: Size = Size::new(88, 88);

/// The most pages a screen may have: their dots fit the left edge, and each
/// has its own recording hash code.
pub const MAX_PAGES: u8 = 12;

/// The Launcher's tiles, in order: apps first, Settings last.
pub const LAUNCHER_TILES: [Tile; 2] = [Tile::Timer, Tile::Settings];

/// The Settings screen's tiles, in order: Wade's look, the chime and
/// brightness, then the information button.
pub const SETTINGS_TILES: [SettingsTile; 5] = [
    SettingsTile::Toggle(SettingsButton::EyeStyle),
    SettingsTile::Toggle(SettingsButton::Color),
    SettingsTile::Toggle(SettingsButton::Chime),
    SettingsTile::Toggle(SettingsButton::Brightness),
    SettingsTile::About,
];

const _: () = assert!(LAUNCHER_TILES.len() <= MAX_PAGES as usize * GRID.len());
const _: () = assert!(SETTINGS_TILES.len() <= MAX_PAGES as usize * GRID.len());

/// How many pages `tiles` fill: at least one, at most `MAX_PAGES`.
#[must_use]
pub const fn page_count<T>(tiles: &[T]) -> u8 {
    let mut count = 1;
    while count < MAX_PAGES && count as usize * GRID.len() < tiles.len() {
        count += 1;
    }
    count
}

/// The tiles on `page`, each with its slot in the grid. A page past the
/// last has none.
pub fn page_tiles<T: Copy>(tiles: &[T], page: u8) -> impl Iterator<Item = (T, Rectangle)> {
    tiles
        .chunks(GRID.len())
        .nth(usize::from(page))
        .unwrap_or_default()
        .iter()
        .copied()
        .zip(GRID)
}

/// The page that shows `tile` and its slot there, if `tiles` holds it.
#[must_use]
pub fn find_tile<T: PartialEq>(tiles: &[T], tile: &T) -> Option<(u8, Rectangle)> {
    let index = tiles.iter().position(|t| t == tile)?;
    let page = u8::try_from(index / GRID.len()).ok()?;
    Some((page, GRID[index % GRID.len()]))
}

/// The page arrows of a screen showing `page` of `count`: each one's target,
/// area, and whether another page lies that way. Drawing and hit-testing
/// both read this, so an arrow takes taps exactly when it is drawn enabled.
#[must_use]
pub const fn page_arrows(page: u8, count: u8) -> [(Target, Rectangle, bool); 2] {
    [
        (Target::PreviousPage, PREVIOUS_PAGE, page > 0),
        (Target::NextPage, NEXT_PAGE, page < count.saturating_sub(1)),
    ]
}

/// Where the page indicator's dots go for `count` pages: stacked
/// vertically on the left edge, centered. One page has none.
pub fn page_dots(count: u8) -> impl Iterator<Item = Point> {
    let shown = if count > 1 { count } else { 0 };
    let top = 118 - 6 * (i32::from(shown) - 1);
    (0..i32::from(shown)).map(move |i| Point::new(22, top + 12 * i))
}

/// A tile on the Settings screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsTile {
    Toggle(SettingsButton),
    /// The information button, which opens About.
    About,
}

impl From<SettingsTile> for Target {
    fn from(tile: SettingsTile) -> Target {
        match tile {
            SettingsTile::Toggle(button) => Target::Settings(button),
            SettingsTile::About => Target::About,
        }
    }
}

impl From<Tile> for Target {
    fn from(tile: Tile) -> Target {
        Target::Tile(tile)
    }
}

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
    About,
    PreviousPage,
    NextPage,
    Tile(Tile),
    Timer(TimerButton),
    Settings(SettingsButton),
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

/// The target under `point` on the Launcher's `page`, if any.
#[must_use]
pub fn hit_launcher(point: Point, page: u8) -> Option<Target> {
    hit_paged(point, &LAUNCHER_TILES, page)
}

/// The target under `point` on the Settings screen's `page`, if any.
#[must_use]
pub fn hit_settings(point: Point, page: u8) -> Option<Target> {
    hit_paged(point, &SETTINGS_TILES, page)
}

/// The target under `point` on a screen of paged `tiles` showing `page`:
/// back, an available page arrow, or a tile.
fn hit_paged<T: Copy + Into<Target>>(point: Point, tiles: &[T], page: u8) -> Option<Target> {
    if BACK.contains(point) {
        return Some(Target::Back);
    }
    let arrow = page_arrows(page, page_count(tiles))
        .into_iter()
        .find(|&(_, area, enabled)| enabled && area.contains(point));
    if let Some((arrow, _, _)) = arrow {
        return Some(arrow);
    }
    page_tiles(tiles, page)
        .find(|(_, area)| area.contains(point))
        .map(|(tile, _)| tile.into())
}

/// About has only a back button; its metadata takes no touches.
#[must_use]
pub fn hit_about(point: Point) -> Option<Target> {
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

    /// A hit area on a screen and page, with a target that uses it, and the
    /// navigation corner it owns, if any.
    type HitArea = ((Screen, u8), Target, Rectangle, Option<Rectangle>);

    /// Every hit area on every screen, taken from the layout's own tables so
    /// that a new one is checked too.
    fn hit_areas() -> Vec<HitArea> {
        let mut areas = vec![
            ((Screen::Buddy, 0), Target::Wade, WADE_FACE, None),
            ((Screen::Buddy, 0), Target::Apps, APPS, Some(APPS)),
            ((Screen::Timer, 0), Target::Back, BACK, Some(BACK)),
            ((Screen::About, 0), Target::Back, BACK, Some(BACK)),
        ];
        paged_areas(&mut areas, Screen::Launcher, &LAUNCHER_TILES);
        paged_areas(&mut areas, Screen::Settings, &SETTINGS_TILES);
        let row = TimerState::new().row();
        areas.extend(TIMER_ROW.iter().zip(row).filter_map(|(&area, button)| {
            button.map(|b| ((Screen::Timer, 0), Target::Timer(b.button), area, None))
        }));
        areas
    }

    /// The hit areas on every page of a screen of paged `tiles`.
    fn paged_areas<T: Copy + Into<Target>>(areas: &mut Vec<HitArea>, screen: Screen, tiles: &[T]) {
        let count = page_count(tiles);
        for page in 0..count {
            let key = (screen, page);
            areas.push((key, Target::Back, BACK, Some(BACK)));
            for (arrow, area, enabled) in page_arrows(page, count) {
                if enabled {
                    areas.push((key, arrow, area, None));
                }
            }
            areas
                .extend(page_tiles(tiles, page).map(|(tile, area)| (key, tile.into(), area, None)));
        }
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
        let launcher = |p| hit_launcher(p, 0);
        for hit in [launcher, timer] {
            assert_eq!(hit(APPS.center()), None);
            assert_eq!(hit(BACK.center()), Some(Target::Back));
        }
        for page in 0..page_count(&SETTINGS_TILES) {
            assert_eq!(hit_settings(BACK.center(), page), Some(Target::Back));
            assert_eq!(hit_settings(APPS.center(), page), None);
        }
        assert_eq!(hit_buddy(BACK.center()), None);
    }

    #[test]
    fn settings_is_the_last_tile() {
        assert_eq!(LAUNCHER_TILES.last(), Some(&Tile::Settings));
        for tile in LAUNCHER_TILES {
            let (page, area) = find_tile(&LAUNCHER_TILES, &tile).unwrap();
            assert_eq!(hit_launcher(area.center(), page), Some(Target::Tile(tile)));
        }
    }

    #[test]
    fn page_arrows_hit_only_where_another_page_lies() {
        let last = page_count(&SETTINGS_TILES) - 1;
        assert_eq!(hit_settings(PREVIOUS_PAGE.center(), 0), None);
        assert_eq!(hit_settings(NEXT_PAGE.center(), 0), Some(Target::NextPage));
        assert_eq!(
            hit_settings(PREVIOUS_PAGE.center(), last),
            Some(Target::PreviousPage)
        );
        assert_eq!(hit_settings(NEXT_PAGE.center(), last), None);
        assert_eq!(page_count(&LAUNCHER_TILES), 1);
        assert_eq!(hit_launcher(PREVIOUS_PAGE.center(), 0), None);
        assert_eq!(hit_launcher(NEXT_PAGE.center(), 0), None);
    }

    #[test]
    fn nine_tiles_fill_two_pages_and_part_of_a_third() {
        let tiles: Vec<u8> = (0..9).collect();
        assert_eq!(page_count(&tiles), 3);
        let page = |n| page_tiles(&tiles, n).map(|(t, _)| t).collect::<Vec<_>>();
        assert_eq!(page(1), [4, 5, 6, 7]);
        assert_eq!(page(2), [8]);
        assert_eq!(
            page_tiles(&tiles, 2).next().map(|(_, area)| area),
            Some(GRID[0])
        );
        assert!(page(3).is_empty(), "a page past the last has no tiles");
        assert_eq!(find_tile(&tiles, &8), Some((2, GRID[0])));
        assert_eq!(find_tile(&tiles, &9), None);
        let enabled = |page| page_arrows(page, 3).map(|(_, _, enabled)| enabled);
        assert_eq!(enabled(0), [false, true]);
        assert_eq!(enabled(1), [true, true]);
        assert_eq!(enabled(2), [true, false]);
    }

    #[test]
    fn page_counts_and_arrows_hold_at_the_limits() {
        assert_eq!(page_count::<u8>(&[]), 1);
        assert_eq!(page_count(&[0u8; 4]), 1);
        assert_eq!(page_count(&[0u8; 5]), 2);
        assert_eq!(page_count(&[0u8; 1_000]), MAX_PAGES);
        assert_eq!(
            page_arrows(u8::MAX, u8::MAX).map(|(_, _, e)| e),
            [true, false]
        );
        assert_eq!(page_arrows(0, 0).map(|(_, _, e)| e), [false, false]);
    }

    #[test]
    fn a_page_past_the_last_hits_no_tiles() {
        let page = page_count(&SETTINGS_TILES) + 3;
        for area in GRID {
            assert_eq!(hit_settings(area.center(), page), None);
        }
        assert_eq!(hit_settings(BACK.center(), page), Some(Target::Back));
        assert_eq!(hit_settings(NEXT_PAGE.center(), page), None);
    }

    #[test]
    fn max_pages_of_dots_fit_between_the_corners() {
        let dots: Vec<Point> = page_dots(MAX_PAGES).collect();
        assert_eq!(dots.len(), usize::from(MAX_PAGES));
        for dot in dots {
            let area = Rectangle::new(dot, Size::new(4, 4));
            assert!(
                NAV_CORNERS
                    .iter()
                    .all(|c| c.intersection(&area).is_zero_sized())
            );
        }
    }

    #[test]
    fn page_dots_center_on_the_left_edge_and_hide_for_one_page() {
        assert_eq!(page_dots(1).count(), 0);
        let dots: Vec<Point> = page_dots(2).collect();
        assert_eq!(dots, [Point::new(22, 112), Point::new(22, 124)]);
        let dots: Vec<Point> = page_dots(3).collect();
        assert_eq!(
            dots,
            [
                Point::new(22, 106),
                Point::new(22, 118),
                Point::new(22, 130)
            ]
        );
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
