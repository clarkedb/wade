//! Wade's face: two eyes shaped by cutting lids out of them, pupils unless the
//! eyes are plain, and an accent beside them, driven entirely by `Pose`
//! (docs/character.md#appearance).

use core::f32::consts::PI;

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Line, Polyline, PrimitiveStyle, Rectangle, RoundedRectangle, Triangle},
};

use crate::character::{Accent, Pose};
use crate::layout;
use crate::render::palette;
use crate::view::EyeStyle;

/// Space between the eyes.
const EYE_GAP: f32 = 40.0;
/// How far plain eyes move at full gaze, per axis.
const GAZE_RANGE: (f32, f32) = (20.0, 12.5);
/// With pupils, the pupils do most of the looking and the eyes follow this much.
const EYE_FOLLOW: f32 = 0.4;
/// Pupils travel this fraction of the room inside the eye.
const PUPIL_TRAVEL: f32 = 0.75;
/// Openings between the lids shorter than this hide the pupils, so a closed
/// eye stays a line.
const PUPIL_MIN_OPENING: f32 = 12.0;
/// The highlight on each pupil: its size, and its offset up and left, as
/// fractions of the pupil's diameter.
const HIGHLIGHT_SIZE: f32 = 0.26;
const HIGHLIGHT_OFFSET: f32 = 0.2;
/// The deepest brow cut, as a fraction of eye height.
const BROW_CUT: f32 = 0.45;
/// How much taller the right eye is at full asymmetry.
const ASYMMETRY: f32 = 0.25;
/// A closed eye is a line this thick.
const MIN_EYE_HEIGHT: f32 = 4.0;
/// Minimum line width (docs/character.md#appearance).
const LINE: u32 = 3;
/// Steam, and the largest Z.
const THICK_LINE: u32 = 4;
/// The "!": its bar's top-left corner, size, and corner radius, and its dot's center.
const EXCLAIM_BAR: (f32, f32, f32, f32) = (287.0, 12.0, 10.0, 28.0);
const EXCLAIM_RADIUS: f32 = 4.0;
const EXCLAIM_DOT: (f32, f32) = (292.0, 50.0);
/// Thinking's dots: the center of each, in the order they appear.
const DOTS: [(f32, f32); 3] = [(255.0, 22.0), (275.0, 22.0), (295.0, 22.0)];
const DOT_DIAMETER: u32 = 12;
/// Where each Z starts rising, at its top-left corner.
const Z_START: (f32, f32) = (240.0, 88.0);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

/// Where one eye is drawn this frame.
#[derive(Clone, Copy)]
struct Eye {
    center: (f32, f32),
    /// Drawn size, after blinking
    size: (f32, f32),
    side: Side,
}

/// Both eyes, and the top and bottom of the unblinked eyes, which accents anchor to.
struct Face {
    left: Eye,
    right: Eye,
    top: f32,
    bottom: f32,
}

impl Face {
    fn new(pose: &Pose, style: EyeStyle) -> Face {
        let (w, h) = pose.eye_size;
        let follow = match style {
            EyeStyle::Pupils => EYE_FOLLOW,
            EyeStyle::Plain => 1.0,
        };
        let x =
            coord(layout::WADE_CENTER.x) + pose.gaze.0 * GAZE_RANGE.0 * follow + pose.face_offset.0;
        let y =
            coord(layout::WADE_CENTER.y) + pose.gaze.1 * GAZE_RANGE.1 * follow + pose.face_offset.1;
        let reach = f32::midpoint(EYE_GAP, w);
        let eye = |cx: f32, height: f32, side| Eye {
            center: (cx, y),
            size: (w, (height * pose.eye_open).max(MIN_EYE_HEIGHT)),
            side,
        };
        Face {
            left: eye(x - reach, h, Side::Left),
            right: eye(
                x + reach,
                h * (1.0 + ASYMMETRY * pose.asymmetry),
                Side::Right,
            ),
            top: y - h / 2.0,
            bottom: y + h / 2.0,
        }
    }

    /// The outer edges of the left and right eyes.
    fn outer(&self) -> (f32, f32) {
        (
            self.left.center.0 - self.left.size.0 / 2.0,
            self.right.center.0 + self.right.size.0 / 2.0,
        )
    }
}

pub fn draw<D>(pose: &Pose, style: EyeStyle, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let face = Face::new(pose, style);
    draw_eye(pose, style, face.left, target)?;
    draw_eye(pose, style, face.right, target)?;
    draw_accent(pose, &face, target)
}

fn draw_eye<D>(pose: &Pose, style: EyeStyle, eye: Eye, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let (w, h) = eye.size;
    let (x, y) = (eye.center.0 - w / 2.0, eye.center.1 - h / 2.0);
    let radius = pose.eye_radius.min(w.min(h) / 2.0 - 1.0).max(0.0);
    RoundedRectangle::with_equal_corners(rect(x, y, w, h), size(radius, radius))
        .into_styled(PrimitiveStyle::with_fill(palette::EYE))
        .draw(target)?;

    // Pupils go in before the lids are cut. Each sits in the opening between
    // the lids and is no taller than it.
    let (top, bottom) = (y + pose.upper_lid * h, y + h - pose.lower_lid * h);
    let opening = bottom - top;
    if style == EyeStyle::Pupils && opening >= PUPIL_MIN_OPENING {
        let pupil = pose.pupil.min(opening);
        let reach = |room: f32| PUPIL_TRAVEL * ((room - pupil) / 2.0).max(0.0);
        let center = (
            eye.center.0 + pose.gaze.0 * reach(w),
            f32::midpoint(top, bottom) + pose.gaze.1 * reach(opening),
        );
        let offset = HIGHLIGHT_OFFSET * pupil;
        let mut inside = target.clipped(&rect(x, y, w, h));
        Circle::with_center(point(center.0, center.1), length(pupil))
            .into_styled(PrimitiveStyle::with_fill(palette::BACKGROUND))
            .draw(&mut inside)?;
        Circle::with_center(
            point(center.0 - offset, center.1 - offset),
            length(HIGHLIGHT_SIZE * pupil),
        )
        .into_styled(PrimitiveStyle::with_fill(palette::EYE))
        .draw(&mut inside)?;
    }

    // The rest is cut out of the eye in the background color.
    let cut = PrimitiveStyle::with_fill(palette::BACKGROUND);

    // Slant: a triangle cut across the top, deepest at one corner. Positive
    // tilt cuts the inner corners (angry), negative the outer ones (worried).
    let depth = pose.brow_tilt.abs() * BROW_CUT * h;
    if depth >= 1.0 {
        let (x0, x1) = (x - 1.0, x + w + 1.0);
        let deep = match (eye.side, pose.brow_tilt > 0.0) {
            (Side::Left, true) | (Side::Right, false) => x1,
            (Side::Left, false) | (Side::Right, true) => x0,
        };
        Triangle::new(
            point(x0, y - 1.0),
            point(x1, y - 1.0),
            point(deep, y + depth),
        )
        .into_styled(cut)
        .draw(target)?;
    }

    // Lower lid: a wide disc risen from below leaves a smiling crescent.
    let lower = pose.lower_lid * h;
    if lower >= 1.0 {
        Circle::with_center(point(eye.center.0, y + h + w - lower), length(2.0 * w))
            .into_styled(cut)
            .draw(target)?;
    }

    // Upper lid: a flat droop from above.
    let upper = pose.upper_lid * h;
    if upper >= 1.0 {
        rect(x - 1.0, y - 1.0, w + 2.0, upper + 1.0)
            .into_styled(cut)
            .draw(target)?;
    }

    Ok(())
}

fn draw_accent<D>(pose: &Pose, face: &Face, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let phase = pose.accent_phase;
    let (outer_left, outer_right) = face.outer();
    let fill = PrimitiveStyle::with_fill(palette::EYE);
    match pose.accent {
        Accent::None => Ok(()),
        Accent::Blush => {
            let style = PrimitiveStyle::with_stroke(palette::BLUSH, LINE);
            for dx in [0.0, 8.0] {
                for x in [outer_left - 20.0 + dx, outer_right + 8.0 + dx] {
                    line(x, face.bottom + 2.0, x + 8.0, face.bottom - 8.0)
                        .into_styled(style)
                        .draw(target)?;
                }
            }
            Ok(())
        }
        Accent::Exclaim => {
            let (x, y, w, h) = EXCLAIM_BAR;
            RoundedRectangle::with_equal_corners(
                rect(x, y, w, h),
                size(EXCLAIM_RADIUS, EXCLAIM_RADIUS),
            )
            .into_styled(fill)
            .draw(target)?;
            Circle::with_center(point(EXCLAIM_DOT.0, EXCLAIM_DOT.1), length(w))
                .into_styled(fill)
                .draw(target)
        }
        Accent::Tear => {
            let start = face.bottom + 12.0;
            let end = coord(layout::SCREEN_SIZE.height.cast_signed()) + 20.0;
            water_drop(outer_right - 8.0, start + (end - start) * phase, target)
        }
        Accent::Dots => {
            let count = usize::try_from(px(phase * 3.0)).unwrap_or(0);
            for &(x, y) in DOTS.iter().take(count) {
                Circle::with_center(point(x, y), DOT_DIAMETER)
                    .into_styled(fill)
                    .draw(target)?;
            }
            Ok(())
        }
        Accent::Steam => {
            let style = PrimitiveStyle::with_stroke(palette::EYE, THICK_LINE);
            line(
                outer_left - 10.0,
                face.top - 10.0,
                outer_left - 5.0,
                face.top - 22.0,
            )
            .into_styled(style)
            .draw(target)?;
            line(
                outer_right + 10.0,
                face.top - 10.0,
                outer_right + 5.0,
                face.top - 22.0,
            )
            .into_styled(style)
            .draw(target)
        }
        Accent::Sparkle => {
            let reach = 11.0 * libm::sinf(PI * phase);
            if reach < 1.0 {
                return Ok(());
            }
            let (cx, cy) = (outer_right + 14.0, face.top - 6.0);
            let style = PrimitiveStyle::with_stroke(palette::SPARKLE, LINE);
            line(cx - reach, cy, cx + reach, cy)
                .into_styled(style)
                .draw(target)?;
            line(cx, cy - reach, cx, cy + reach)
                .into_styled(style)
                .draw(target)
        }
        Accent::SweatDrop => water_drop(outer_left - 16.0, face.top + 16.0 + 36.0 * phase, target),
        Accent::Zs => {
            let newest = phase - libm::floorf(phase);
            draw_z(newest / 2.0, target)?;
            if phase >= 1.0 {
                draw_z(f32::midpoint(newest, 1.0), target)?;
            }
            Ok(())
        }
    }
}

/// A drop of water, point up, its round bottom centered on (x, y).
fn water_drop<D>(x: f32, y: f32, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let fill = PrimitiveStyle::with_fill(palette::WATER);
    Triangle::new(
        point(x - 5.0, y - 2.0),
        point(x + 5.0, y - 2.0),
        point(x, y - 15.0),
    )
    .into_styled(fill)
    .draw(target)?;
    Circle::with_center(point(x, y), 12)
        .into_styled(fill)
        .draw(target)
}

/// One Z, `age` 0.0 … 1.0 through its rise: it drifts up and right, grows, and
/// flickers out.
fn draw_z<D>(age: f32, target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    if age > 0.8 && px(age * 20.0) % 2 == 1 {
        return Ok(());
    }
    let (x, y) = (Z_START.0 + 45.0 * age, Z_START.1 - 66.0 * age);
    let (w, h, width) = if age < 1.0 / 3.0 {
        (14.0, 18.0, LINE)
    } else if age < 2.0 / 3.0 {
        (18.0, 24.0, LINE)
    } else {
        (22.0, 30.0, THICK_LINE)
    };
    Polyline::new(&[
        point(x, y),
        point(x + w, y),
        point(x, y + h),
        point(x + w, y + h),
    ])
    .into_styled(PrimitiveStyle::with_stroke(palette::EYE, width))
    .draw(target)
}

/// The nearest whole pixel.
#[expect(
    clippy::cast_possible_truncation,
    reason = "float-to-int `as` saturates, and pixel coordinates are far inside i32"
)]
fn px(v: f32) -> i32 {
    libm::roundf(v) as i32
}

/// The nearest whole length in pixels, zero if negative.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "float-to-int `as` saturates, clamping negative lengths to zero"
)]
fn length(v: f32) -> u32 {
    libm::roundf(v) as u32
}

#[expect(
    clippy::cast_precision_loss,
    reason = "screen coordinates are far below 2^24"
)]
fn coord(v: i32) -> f32 {
    v as f32
}

fn point(x: f32, y: f32) -> Point {
    Point::new(px(x), px(y))
}

fn size(w: f32, h: f32) -> Size {
    Size::new(length(w), length(h))
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
    Rectangle::new(point(x, y), size(w, h))
}

fn line(x0: f32, y0: f32, x1: f32, y1: f32) -> Line {
    Line::new(point(x0, y0), point(x1, y1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::{ASLEEP, Wade};
    use crate::time::Instant;

    #[test]
    fn a_sleeping_eye_never_opens_far_enough_to_show_pupils() {
        let ms = Instant::from_millis;
        let mut wade = Wade::new(ms(0), 1);
        let _ = wade.sleep(ms(0));
        let mut twitched = false;
        // A twitch opens the eyes widest as it starts, which is a transition.
        while let Some(t) = wade.next_transition(true).filter(|&t| t <= ms(120_000)) {
            let _ = wade.advance(t, true);
            let pose = wade.pose(t);
            if t < ms(2_000) {
                continue; // still falling asleep
            }
            let face = Face::new(&pose, EyeStyle::Pupils);
            for eye in [face.left, face.right] {
                let opening = eye.size.1 * (1.0 - pose.upper_lid - pose.lower_lid);
                assert!(opening < PUPIL_MIN_OPENING, "{opening} px open at {t:?}");
            }
            twitched |= pose.eye_size.1 > ASLEEP.eye_size.1;
        }
        assert!(twitched, "no twitch in two minutes asleep");
    }
}
