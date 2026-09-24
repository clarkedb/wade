//! Desktop simulator for Wade (docs/platforms.md#desktop-wade-desktop).
//!
//! Runs the platform loop from docs/architecture.md#core-api against an
//! `embedded-graphics-simulator` window.

mod clock;

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
    sdl2::{Keycode, MouseButton},
};
use wade_core::{App, Digit, Event, Key, TouchPhase, layout, render};

use clock::Clock;

/// Longest the loop sleeps before polling window events again.
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Parser, Debug)]
#[command(version, about = "Wade desktop simulator")]
struct Args {
    /// Fixed random seed. Defaults to OS entropy.
    #[arg(long, conflicts_with = "replay")]
    seed: Option<u64>,

    /// Write the seed and every input event to a recording.
    #[arg(long, value_name = "FILE", conflicts_with = "replay")]
    record: Option<PathBuf>,

    /// Play a recording back in real time instead of reading the mouse.
    #[arg(long, value_name = "FILE")]
    replay: Option<PathBuf>,

    /// Run the clock this many times faster (0.01 to 1000).
    #[arg(long, value_name = "X", default_value_t = 1.0, value_parser = parse_time_scale)]
    time_scale: f64,
}

fn parse_time_scale(s: &str) -> Result<f64, String> {
    let x: f64 = s.parse().map_err(|e| format!("{e}"))?;
    if (0.01..=1_000.0).contains(&x) {
        Ok(x)
    } else {
        Err("must be between 0.01 and 1000".into())
    }
}

fn main() {
    let args = Args::parse();
    if args.record.is_some() {
        todo!("M1: --record");
    }
    if args.replay.is_some() {
        todo!("M1: --replay");
    }

    let seed = args
        .seed
        .unwrap_or_else(|| getrandom::u64().expect("OS entropy"));
    eprintln!("seed {seed}");
    eprintln!("keys: 0–9 show an expression, z sleeps, p toggles pupils; click Wade to tap him");

    let clock = Clock::new(args.time_scale);
    let mut app = App::new(clock.now(), seed);

    let mut display = SimulatorDisplay::<Rgb565>::new(layout::SCREEN_SIZE);
    let settings = OutputSettingsBuilder::new().scale(2).build();
    let mut window = Window::new("Wade", &settings);
    // The loop decides when to sleep; don't let the window throttle updates.
    window.set_max_fps(1_000);

    let Ok(()) = render::draw(&app.view(), &mut display);
    window.update(&display);

    let mut mouse_down = false;
    loop {
        let mut redraw = false;

        for sim_event in window.events() {
            let now = clock.now();
            // The simulator reports points in display coordinates (it undoes the scale).
            let event = match sim_event {
                SimulatorEvent::Quit => return,
                SimulatorEvent::KeyDown {
                    keycode,
                    repeat: false,
                    ..
                } => key(keycode).map(|key| Event::key(now, key)),
                SimulatorEvent::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    point,
                } => {
                    mouse_down = true;
                    Some(Event::touch(now, TouchPhase::Down, point))
                }
                SimulatorEvent::MouseMove { point } if mouse_down => {
                    Some(Event::touch(now, TouchPhase::Move, point))
                }
                SimulatorEvent::MouseButtonUp {
                    mouse_btn: MouseButton::Left,
                    point,
                } if mouse_down => {
                    mouse_down = false;
                    Some(Event::touch(now, TouchPhase::Up, point))
                }
                _ => None,
            };
            if let Some(event) = event {
                redraw |= app.handle(event).redraw;
            }
        }

        let now = clock.now();
        if app.next_deadline().is_some_and(|d| d <= now) {
            let output = app.handle(Event::deadline(now));
            redraw |= output.redraw;
        }

        if redraw {
            let Ok(()) = render::draw(&app.view(), &mut display);
            window.update(&display);
        }

        let sleep = app
            .next_deadline()
            .map_or(POLL_INTERVAL, |d| clock.real_until(d).min(POLL_INTERVAL));
        std::thread::sleep(sleep);
    }
}

/// The core key for a keyboard key, if it has one.
fn key(keycode: Keycode) -> Option<Key> {
    let n = match keycode {
        Keycode::Z => return Some(Key::Z),
        Keycode::P => return Some(Key::P),
        Keycode::NUM_0 => 0,
        Keycode::NUM_1 => 1,
        Keycode::NUM_2 => 2,
        Keycode::NUM_3 => 3,
        Keycode::NUM_4 => 4,
        Keycode::NUM_5 => 5,
        Keycode::NUM_6 => 6,
        Keycode::NUM_7 => 7,
        Keycode::NUM_8 => 8,
        Keycode::NUM_9 => 9,
        _ => return None,
    };
    Digit::new(n).map(Key::Digit)
}
