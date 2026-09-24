//! Rendered-pixel snapshots (docs/testing.md#snapshots).
//!
//! `UPDATE_SNAPSHOTS=1 cargo test` rewrites the golden images; review the diff.

mod common;

use common::framebuffer::{Framebuffer, assert_snapshot};
use wade_core::character::{ASLEEP, Accent, Expression, Pose};
use wade_core::harness::Harness;
use wade_core::view::{BuddyView, EyeStyle, View};
use wade_core::{App, Instant, render};

fn snapshot(name: &str, view: &View) {
    let mut fb = Framebuffer::new();
    let Ok(()) = render::draw(view, &mut fb);
    assert_snapshot(name, &fb);
}

/// Both eye styles, with the prefix of their snapshot names.
const STYLES: [(EyeStyle, &str); 2] = [(EyeStyle::Pupils, "pupils"), (EyeStyle::Plain, "plain")];

fn buddy(expression: Expression, asleep: bool, eye_style: EyeStyle, pose: Pose) -> View {
    View::Buddy(BuddyView {
        expression,
        asleep,
        blinking: false,
        pose,
        eye_style,
    })
}

/// A phase that shows the accent clearly in a still image.
fn still_phase(accent: Accent) -> f32 {
    match accent {
        Accent::None | Accent::Blush | Accent::Dots | Accent::Steam => 1.0,
        Accent::Exclaim | Accent::Sparkle => 0.5,
        Accent::Tear => 0.3,
        Accent::SweatDrop => 0.4,
        Accent::Zs => 1.3,
    }
}

#[test]
fn buddy_at_start() {
    let app = App::new(Instant::from_millis(0), common::SEED);
    snapshot("buddy_at_start", &app.view());
}

#[test]
fn buddy_once_awake() {
    let mut h = Harness::new(common::SEED);
    h.run_until(common::ms(1_000));
    snapshot("buddy_once_awake", &h.app.view());
}

#[test]
fn each_expression_at_rest() {
    for (eye_style, prefix) in STYLES {
        for (number, expression) in Expression::ALL.into_iter().enumerate() {
            let accent = expression.accent();
            let pose = Pose {
                accent,
                accent_phase: still_phase(accent),
                ..expression.pose()
            };
            let name = format!("{expression:?}").to_lowercase();
            snapshot(
                &format!("{prefix}_{number}_{name}"),
                &buddy(expression, false, eye_style, pose),
            );
        }
    }
}

#[test]
fn glancing() {
    let pose = Pose {
        gaze: (-0.8, 0.5),
        ..Expression::Neutral.pose()
    };
    for (eye_style, prefix) in STYLES {
        snapshot(
            &format!("{prefix}_glancing"),
            &buddy(Expression::Neutral, false, eye_style, pose),
        );
    }
}

#[test]
fn asleep() {
    let pose = Pose {
        accent: Accent::Zs,
        accent_phase: still_phase(Accent::Zs),
        ..ASLEEP
    };
    snapshot(
        "asleep",
        &buddy(Expression::Sleepy, true, EyeStyle::Pupils, pose),
    );
}

#[test]
fn mid_blink() {
    let pose = Pose {
        eye_open: 0.3,
        ..Expression::Neutral.pose()
    };
    snapshot(
        "mid_blink",
        &buddy(Expression::Neutral, false, EyeStyle::Pupils, pose),
    );
}
