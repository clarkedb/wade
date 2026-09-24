//! Rendered-pixel snapshots (docs/testing.md#snapshots).
//!
//! `UPDATE_SNAPSHOTS=1 cargo test` rewrites the golden images; review the diff.

mod common;

use common::framebuffer::{Framebuffer, assert_snapshot};
use wade_core::character::{Expression, Pose};
use wade_core::view::{BuddyView, View};
use wade_core::{App, Instant, render};

fn draw(view: &View) -> Framebuffer {
    let mut fb = Framebuffer::new();
    let Ok(()) = render::draw(view, &mut fb);
    fb
}

fn buddy(expression: Expression, pose: Pose) -> View {
    View::Buddy(BuddyView {
        expression,
        blinking: false,
        pose,
    })
}

#[test]
fn buddy_at_start() {
    let app = App::new(Instant::from_millis(0), common::SEED);
    assert_snapshot("buddy_at_start", &draw(&app.view()));
}

#[test]
fn buddy_happy() {
    let view = buddy(Expression::Happy, Expression::Happy.pose());
    assert_snapshot("buddy_happy", &draw(&view));
}

#[test]
fn buddy_mid_blink() {
    let pose = Pose {
        eye_open: 0.1,
        ..Expression::Neutral.pose()
    };
    let view = View::Buddy(BuddyView {
        expression: Expression::Neutral,
        blinking: true,
        pose,
    });
    assert_snapshot("buddy_mid_blink", &draw(&view));
}
