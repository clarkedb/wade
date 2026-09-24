//! Rendered-pixel snapshots (docs/testing.md#snapshots).
//!
//! `UPDATE_SNAPSHOTS=1 cargo test` rewrites the golden images; review the diff.

mod common;

use common::framebuffer::{Framebuffer, assert_snapshot};
use wade_core::{App, Instant, render};

#[test]
fn buddy_at_start() {
    let app = App::new(Instant::from_millis(0), common::SEED);
    let mut fb = Framebuffer::new();
    let Ok(()) = render::draw(&app.view(), &mut fb);
    assert_snapshot("buddy_at_start", &fb);
}
