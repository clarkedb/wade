//! Rendered-pixel snapshots (docs/testing.md#snapshots).
//!
//! `UPDATE_SNAPSHOTS=1 cargo test` rewrites the golden images; review the diff.

mod common;

use common::framebuffer::{Framebuffer, assert_snapshot};
use std::time::Duration;
use wade_core::character::{ASLEEP, Accent, Expression, Pose};
use wade_core::harness::Harness;
use wade_core::layout::{self, Target, Tile};
use wade_core::settings::SettingsButton;
use wade_core::timer::{MAX_SET, MIN_SET, TimerButton, TimerState};
use wade_core::view::{ColorMode, EyeStyle, SettingsView, View};
use wade_core::{App, Instant, Settings, TouchPhase, render};

/// The whole screen for `view`.
fn snapshot(name: &str, view: &View) {
    let mut fb = Framebuffer::new();
    let Ok(()) = render::draw(view, &mut fb);
    assert_snapshot(name, &fb);
}

/// Wade's face alone, without the rest of the Buddy screen.
fn face(name: &str, pose: &Pose, eye_style: EyeStyle) {
    face_in(name, pose, eye_style, ColorMode::Color);
}

fn face_in(name: &str, pose: &Pose, eye_style: EyeStyle, color: ColorMode) {
    let mut fb = Framebuffer::new();
    let Ok(()) = render::face::draw(pose, eye_style, color, &mut fb);
    assert_snapshot(name, &fb);
}

/// `expression` at rest with its accent showing.
fn at_rest(expression: Expression) -> Pose {
    let accent = expression.accent();
    Pose {
        accent,
        accent_phase: still_phase(accent),
        ..expression.pose()
    }
}

/// Both eye styles, with the prefix of their snapshot names.
const STYLES: [(EyeStyle, &str); 2] = [(EyeStyle::Pupils, "pupils"), (EyeStyle::Plain, "plain")];

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
    let app = App::new(Instant::from_millis(0), common::SEED, Settings::DEFAULT);
    snapshot("buddy_at_start", &app.view());
}

#[test]
fn buddy_once_awake() {
    let mut h = Harness::new(common::SEED);
    h.run_until(common::ms(1_000));
    snapshot("buddy_once_awake", &h.app.view());
}

#[test]
fn buddy_apps_pressed() {
    let mut h = Harness::new(common::SEED);
    h.touch(common::ms(1_000), TouchPhase::Down, common::apps());
    snapshot("buddy_apps_pressed", &h.app.view());
}

#[test]
fn each_expression_at_rest() {
    for (eye_style, prefix) in STYLES {
        for (number, expression) in Expression::ALL.into_iter().enumerate() {
            let name = format!("{expression:?}").to_lowercase();
            face(
                &format!("{prefix}_{number}_{name}"),
                &at_rest(expression),
                eye_style,
            );
        }
    }
}

#[test]
fn each_colored_accent_in_mono() {
    for expression in [
        Expression::Happy,
        Expression::Sad,
        Expression::Proud,
        Expression::Flustered,
    ] {
        let number = Expression::ALL
            .iter()
            .position(|&e| e == expression)
            .expect("every expression is numbered");
        let name = format!("{expression:?}").to_lowercase();
        face_in(
            &format!("mono_{number}_{name}"),
            &at_rest(expression),
            EyeStyle::Pupils,
            ColorMode::Mono,
        );
    }
}

#[test]
fn glancing() {
    let pose = Pose {
        gaze: (-0.8, 0.5),
        ..Expression::Neutral.pose()
    };
    for (eye_style, prefix) in STYLES {
        face(&format!("{prefix}_glancing"), &pose, eye_style);
    }
}

#[test]
fn asleep() {
    let pose = Pose {
        accent: Accent::Zs,
        accent_phase: still_phase(Accent::Zs),
        ..ASLEEP
    };
    face("asleep", &pose, EyeStyle::Pupils);
}

#[test]
fn mid_blink() {
    let pose = Pose {
        eye_open: 0.3,
        ..Expression::Neutral.pose()
    };
    face("mid_blink", &pose, EyeStyle::Pupils);
}

/// The Timer screen for `timer` at `now`, with `pressed` held down.
fn timer(timer: TimerState, now: Instant, pressed: Option<Target>) -> View {
    View::Timer(timer.view(now, pressed))
}

#[test]
fn each_timer_state() {
    let set = Duration::from_mins(5);
    let now = Instant::from_millis(0);
    for (name, state) in [
        ("timer_ready", TimerState::Ready { set }),
        (
            "timer_running",
            TimerState::Running {
                set,
                ends_at: Instant::from_millis(277_000),
            },
        ),
        (
            "timer_paused",
            TimerState::Paused {
                set,
                remaining: Duration::from_secs(86 * 60 + 58),
            },
        ),
        (
            "timer_done",
            TimerState::Done {
                set,
                since: now,
                chimes: 1,
            },
        ),
    ] {
        snapshot(name, &timer(state, now, None));
    }
}

#[test]
fn each_timer_button_pressed() {
    let set = Duration::from_mins(5);
    let now = Instant::from_millis(0);
    let running = TimerState::Running {
        set,
        ends_at: Instant::from_millis(277_000),
    };
    let paused = TimerState::Paused {
        set,
        remaining: Duration::from_secs(86 * 60 + 58),
    };
    let done = TimerState::Done {
        set,
        since: now,
        chimes: 1,
    };
    for (state, pressed) in [
        (TimerState::new(), Target::Back),
        (TimerState::new(), Target::Timer(TimerButton::Minus)),
        (TimerState::new(), Target::Timer(TimerButton::Start)),
        (TimerState::new(), Target::Timer(TimerButton::Plus)),
        (running, Target::Timer(TimerButton::Pause)),
        (paused, Target::Timer(TimerButton::Reset)),
        (paused, Target::Timer(TimerButton::Resume)),
        (done, Target::Timer(TimerButton::Dismiss)),
    ] {
        let name = match pressed {
            Target::Timer(button) => format!("{button:?}").to_lowercase(),
            other => format!("{other:?}").to_lowercase(),
        };
        snapshot(
            &format!("timer_{name}_pressed"),
            &timer(state, now, Some(pressed)),
        );
    }
}

#[test]
fn timer_bounds_dim_their_buttons() {
    let now = Instant::from_millis(0);
    snapshot(
        "timer_shortest",
        &timer(TimerState::Ready { set: MIN_SET }, now, None),
    );
    snapshot(
        "timer_longest",
        &timer(TimerState::Ready { set: MAX_SET }, now, None),
    );
}

#[test]
fn launcher() {
    let mut h = Harness::new(common::SEED);
    h.tap(common::ms(1_000), common::apps());
    snapshot("launcher", &h.app.view());
}

#[test]
fn each_launcher_button_pressed() {
    for (name, point) in [
        ("back", common::back()),
        ("timer", common::tile(Tile::Timer)),
        ("settings", common::tile(Tile::Settings)),
    ] {
        let mut h = Harness::new(common::SEED);
        h.tap(common::ms(1_000), common::apps());
        h.touch(common::ms(2_000), TouchPhase::Down, point);
        snapshot(&format!("launcher_{name}_pressed"), &h.app.view());
    }
}

/// The Settings screen for `settings`, with `pressed` held down.
fn settings_screen(settings: Settings, pressed: Option<Target>) -> View {
    View::Settings(SettingsView { settings, pressed })
}

#[test]
fn settings_screen_with_defaults_and_changed() {
    snapshot("settings", &settings_screen(Settings::DEFAULT, None));
    let changed = Settings::DEFAULT
        .with_eye_style(EyeStyle::Plain)
        .with_color(ColorMode::Mono)
        .with_chime(false);
    snapshot("settings_changed", &settings_screen(changed, None));
}

#[test]
fn each_settings_button_pressed() {
    let pressed = std::iter::once(("back".to_owned(), Target::Back)).chain(
        layout::SETTINGS_BUTTONS.iter().map(|&(button, _)| {
            let name = match button {
                SettingsButton::EyeStyle => "eye_style",
                SettingsButton::Color => "color",
                SettingsButton::Chime => "chime",
            };
            (name.to_owned(), Target::Settings(button))
        }),
    );
    for (name, target) in pressed {
        snapshot(
            &format!("settings_{name}_pressed"),
            &settings_screen(Settings::DEFAULT, Some(target)),
        );
    }
}
