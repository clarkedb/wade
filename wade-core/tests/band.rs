//! Banded drawing matches full-frame drawing pixel for pixel (docs/ui.md#banded-rendering).

mod common;

use std::time::Duration;

use common::framebuffer::Framebuffer;
use embedded_graphics::prelude::*;
use proptest::prelude::*;
use wade_core::character::{ASLEEP, Accent, Expression, Pose};
use wade_core::harness::Harness;
use wade_core::layout::{SCREEN_SIZE, Tile};
use wade_core::render::{self, Band, ROW_BYTES};
use wade_core::timer::TimerState;
use wade_core::view::{BuddyView, ColorMode, EyeStyle, SettingsView};
use wade_core::{Instant, Key, Settings, View};

/// Draw `view` through strips of `rows` rows and check each against the full frame.
fn assert_banded_matches(view: &View, rows: usize) {
    let mut full = Framebuffer::new();
    let Ok(()) = render::draw(view, &mut full);

    // Reused across strips, as a platform would, to show that every pixel is redrawn.
    let mut buf = vec![0; ROW_BYTES * rows];
    let mut covered = 0;
    for top in Band::tops(rows) {
        let mut band = Band::new(&mut buf, top);
        band.draw(view);
        assert_eq!(
            band.area().top_left,
            Point::new(0, i32::try_from(top).unwrap())
        );
        for (i, row) in band.bytes().as_chunks::<ROW_BYTES>().0.iter().enumerate() {
            let y = top as usize + i;
            let expected: Vec<u8> = full
                .row(y)
                .iter()
                .flat_map(|c| c.into_storage().to_be_bytes())
                .collect();
            assert!(
                row[..] == expected[..],
                "{rows}-row band differs at row {y} for {view:?}"
            );
            covered += 1;
        }
    }
    assert_eq!(
        covered, SCREEN_SIZE.height as usize,
        "{rows}-row bands miss rows"
    );
}

/// Band heights that divide the screen, leave a short last band, or exceed it.
fn assert_every_height(view: &View) {
    for rows in [1, 7, 40, 64, 300] {
        assert_banded_matches(view, rows);
    }
}

fn buddy(pose: Pose, eye_style: EyeStyle, color: ColorMode) -> View {
    View::Buddy(BuddyView {
        expression: Expression::Neutral,
        asleep: false,
        blinking: false,
        pose,
        eye_style,
        color,
        apps_pressed: false,
    })
}

#[test]
fn every_expression() {
    for expression in Expression::ALL {
        let pose = Pose {
            accent: expression.accent(),
            accent_phase: 0.5,
            ..expression.pose()
        };
        for eye_style in [EyeStyle::Pupils, EyeStyle::Plain] {
            for color in [ColorMode::Color, ColorMode::Mono] {
                assert_every_height(&buddy(pose, eye_style, color));
            }
        }
    }
}

#[test]
fn asleep_and_mid_blink() {
    let asleep = Pose {
        accent: Accent::Zs,
        accent_phase: 1.3,
        ..ASLEEP
    };
    let blink = Pose {
        eye_open: 0.3,
        ..Expression::Neutral.pose()
    };
    for pose in [asleep, blink] {
        assert_every_height(&buddy(pose, EyeStyle::Pupils, ColorMode::Color));
    }
}

#[test]
fn every_screen() {
    let mut h = Harness::new(common::SEED);
    h.run_until(common::ms(1_000));
    assert_every_height(&h.app.view());
    h.tap(common::ms(1_000), common::apps());
    assert_every_height(&h.app.view());

    let set = Duration::from_mins(5);
    let now = Instant::from_millis(0);
    for state in [
        TimerState::Ready { set },
        TimerState::Running {
            set,
            ends_at: Instant::from_millis(277_000),
        },
        TimerState::Done {
            set,
            since: now,
            chimes: 1,
        },
    ] {
        assert_every_height(&View::Timer(state.view(now, None)));
    }

    assert_every_height(&View::Settings(SettingsView {
        settings: Settings::DEFAULT,
        pressed: None,
    }));
}

/// A tap somewhere, often on a touch target, or a key.
#[derive(Clone, Debug)]
enum Input {
    Tap(Point),
    Key(Key),
}

fn input() -> impl Strategy<Value = Input> {
    let targets = vec![
        common::apps(),
        common::back(),
        common::tile(Tile::Timer),
        common::tile(Tile::Settings),
        wade_core::layout::WADE_CENTER,
    ];
    prop_oneof![
        3 => prop::sample::select(targets).prop_map(Input::Tap),
        1 => (0i32..320, 0i32..240).prop_map(|(x, y)| Input::Tap(Point::new(x, y))),
        1 => prop_oneof![(0u8..10).prop_map(common::digit), Just(Key::Z), Just(Key::P)]
            .prop_map(Input::Key),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn any_session_draws_the_same_in_bands(
        seed: u64,
        inputs in prop::collection::vec((0u64..3_000, input()), 0..12),
    ) {
        let mut h = Harness::new(seed);
        let mut t = 0;
        for (delay, input) in inputs {
            t += delay;
            match input {
                Input::Tap(point) => h.tap(common::ms(t), point),
                Input::Key(key) => h.key(common::ms(t), key),
            }
        }
        let view = h.app.view();
        assert_banded_matches(&view, 7);
        assert_banded_matches(&view, 40);
    }
}
