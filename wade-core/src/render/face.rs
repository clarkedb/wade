//! Wade's placeholder face (M1–M3): a round head, two eyes, two brows, and a mouth,
//! driven entirely by `Pose`.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "small pixel geometry: every value is far inside the range where these casts are exact, and float-to-int `as` saturates"
)]

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{
        Circle, Ellipse, Line, Polyline, PrimitiveStyle, PrimitiveStyleBuilder, StrokeAlignment,
    },
};

use crate::character::Pose;
use crate::layout;
use crate::render::palette;

/// Head outline width; the minimum line width (docs/character.md#appearance).
const LINE: u32 = 3;
/// Stroke width for brows, the mouth, and closed eyes.
const FEATURE: u32 = 4;

/// Horizontal distance from the head centre to each eye centre.
const EYE_SPACING: i32 = 34;
/// Eye centre, relative to the head centre.
const EYE_Y: i32 = -8;
/// Size of a fully open eye.
const EYE_WIDTH: u32 = 26;
const EYE_HEIGHT: u32 = 36;
/// Below this height an open eye would be a sliver, so it is drawn closed.
const EYE_MIN_OPEN_HEIGHT: u32 = 7;
/// Diameter of the highlight on each eye.
const HIGHLIGHT: u32 = 9;
/// How far the eyes shift at full gaze. The eyes have no separate pupils.
const GAZE_RANGE: (f32, f32) = (6.0, 6.0);

/// Brow centre height above the eye centre at rest.
const BROW_ABOVE_EYE: i32 = 30;
/// How far the brows move at full raise.
const BROW_RAISE_RANGE: f32 = 10.0;
const BROW_HALF_WIDTH: i32 = 12;
/// How far each end of a brow moves at full tilt.
const BROW_TILT_RANGE: f32 = 7.0;

/// Mouth centre, relative to the head centre.
const MOUTH_Y: i32 = 34;
const MOUTH_HALF_WIDTH: i32 = 26;
/// Vertical travel of the mouth's centre relative to its corners at full curve.
const MOUTH_CURVE_RANGE: f32 = 16.0;
/// How far each corner of the mouth moves at full skew.
const MOUTH_SKEW_RANGE: f32 = 5.0;
/// Points in the mouth polyline.
const MOUTH_POINTS: usize = 9;

pub fn draw<D>(pose: &Pose, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let center = layout::WADE_CENTER + point(pose.head_offset.0, pose.head_offset.1);

    let head_style = PrimitiveStyleBuilder::new()
        .fill_color(palette::SKIN)
        .stroke_color(palette::OUTLINE)
        .stroke_width(LINE)
        // Keep the outline inside the head, so the drawn head matches its hit area.
        .stroke_alignment(StrokeAlignment::Inside)
        .build();
    Circle::with_center(center, layout::WADE_HEAD_DIAMETER)
        .into_styled(head_style)
        .draw(target)?;

    for side in [-1, 1] {
        let eye = center + Point::new(side * EYE_SPACING, EYE_Y);
        draw_eye(pose, eye, target)?;
        draw_brow(pose, eye, side, target)?;
    }
    draw_mouth(pose, center + Point::new(0, MOUTH_Y), target)
}

fn draw_eye<D>(pose: &Pose, eye: Point, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let open = pose.eye_open.clamp(0.0, 1.0);
    let gaze = point(
        pose.gaze.0.clamp(-1.0, 1.0) * GAZE_RANGE.0,
        pose.gaze.1.clamp(-1.0, 1.0) * GAZE_RANGE.1,
    );
    let eye = eye + gaze;
    let height = libm::roundf(EYE_HEIGHT as f32 * open) as u32;
    if height < EYE_MIN_OPEN_HEIGHT {
        return Line::new(
            eye - Point::new(EYE_WIDTH as i32 / 2, 0),
            eye + Point::new(EYE_WIDTH as i32 / 2, 0),
        )
        .into_styled(PrimitiveStyle::with_stroke(palette::EYE, FEATURE))
        .draw(target);
    }

    // A solid dark oval with a highlight toward the upper left.
    Ellipse::with_center(eye, Size::new(EYE_WIDTH, height))
        .into_styled(PrimitiveStyle::with_fill(palette::EYE))
        .draw(target)?;
    let highlight = HIGHLIGHT.min(height / 2);
    let offset = Point::new(-(EYE_WIDTH as i32) / 5, -(height as i32) / 5);
    Circle::with_center(eye + offset, highlight)
        .into_styled(PrimitiveStyle::with_fill(palette::EYE_HIGHLIGHT))
        .draw(target)
}

/// `side` is −1 for the brow on screen left, 1 for screen right.
fn draw_brow<D>(pose: &Pose, eye: Point, side: i32, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let mut raise = pose.brow_raise.clamp(-1.0, 1.0) * BROW_RAISE_RANGE;
    // Asymmetry lifts the screen-left brow only (skeptical).
    if side < 0 {
        raise += pose.brow_asymmetry.clamp(0.0, 1.0) * BROW_RAISE_RANGE;
    }
    let y = eye.y - BROW_ABOVE_EYE - libm::roundf(raise) as i32;
    // Determined (tilt > 0) lowers the inner ends; worried (tilt < 0) raises them.
    let tilt = libm::roundf(pose.brow_tilt.clamp(-1.0, 1.0) * BROW_TILT_RANGE) as i32;
    let inner = Point::new(eye.x - side * BROW_HALF_WIDTH, y + tilt);
    let outer = Point::new(eye.x + side * BROW_HALF_WIDTH, y - tilt);
    Line::new(inner, outer)
        .into_styled(PrimitiveStyle::with_stroke(palette::OUTLINE, FEATURE))
        .draw(target)
}

fn draw_mouth<D>(pose: &Pose, mouth: Point, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let curve = pose.mouth_curve.clamp(-1.0, 1.0) * MOUTH_CURVE_RANGE;
    let skew = pose.mouth_skew.clamp(-1.0, 1.0);
    let mut points = [Point::zero(); MOUTH_POINTS];
    for (i, p) in points.iter_mut().enumerate() {
        // u runs −1 … 1 across the mouth.
        let u = i as f32 / (MOUTH_POINTS - 1) as f32 * 2.0 - 1.0;
        // A parabola: the centre dips below the corners for a smile, rises for a frown.
        // Skew tilts the line, raising one corner and lowering the other.
        let y = curve * (0.5 - u * u) - skew * u * MOUTH_SKEW_RANGE;
        *p = mouth + point(u * MOUTH_HALF_WIDTH as f32, y);
    }
    // TODO(M1): mouth_open.
    Polyline::new(&points)
        .into_styled(PrimitiveStyle::with_stroke(palette::OUTLINE, FEATURE))
        .draw(target)
}

/// The nearest pixel to `(x, y)`.
fn point(x: f32, y: f32) -> Point {
    Point::new(libm::roundf(x) as i32, libm::roundf(y) as i32)
}
