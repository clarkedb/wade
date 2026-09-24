//! Wade's placeholder face (M1–M3): a round head, two eyes, two brows, and a mouth,
//! driven entirely by `Pose`.

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder},
};

use crate::character::Pose;
use crate::layout;
use crate::render::palette;

/// Minimum outline width (docs/character.md#appearance).
const LINE: u32 = 3;

pub fn draw<D>(pose: &Pose, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let head_style = PrimitiveStyleBuilder::new()
        .fill_color(palette::SKIN)
        .stroke_color(palette::OUTLINE)
        .stroke_width(LINE)
        .build();
    let offset = point(pose.head_offset.0, pose.head_offset.1);
    Circle::with_center(layout::WADE_CENTER + offset, layout::WADE_HEAD_DIAMETER)
        .into_styled(head_style)
        .draw(target)?;

    // TODO(M1): eyes (eye_open, gaze), brows, and mouth (mouth_curve, mouth_open).
    Ok(())
}

/// The nearest pixel to `(x, y)`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "float-to-int `as` saturates, and pixel coordinates are far inside i32"
)]
fn point(x: f32, y: f32) -> Point {
    Point::new(libm::roundf(x) as i32, libm::roundf(y) as i32)
}
