//! Desktop simulator for Wade (docs/platforms.md#desktop-wade-desktop).
//!
//! Runs the platform loop from docs/architecture.md#core-api against an
//! `embedded-graphics-simulator` window.

mod audio;
mod clock;

use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
    sdl2::{Keycode, MouseButton},
};
use wade_core::harness::recording::{Entry, Input, Recording};
use wade_core::harness::state_hash;
use wade_core::{
    App, Digit, Effect, Event, EventKind, Instant, Key, Output, Settings, TouchPhase, layout,
    render,
};

use audio::Audio;
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
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let recording = if let Some(path) = &args.replay {
        let text = fs::read_to_string(path)?;
        Some(
            text.parse::<Recording>()
                .map_err(|error| format!("{}: {error}", path.display()))?,
        )
    } else {
        None
    };

    let seed = if let Some(recording) = &recording {
        recording.seed
    } else if let Some(seed) = args.seed {
        seed
    } else {
        getrandom::u64()?
    };
    eprintln!("seed {seed}");
    if recording.is_none() {
        eprintln!(
            "keys: 0–9 show an expression, z sleeps, p toggles pupils; click Wade to tap him or the bottom-right corner to open the Launcher"
        );
    }

    let audio = Audio::open();
    let clock = Clock::new(args.time_scale);
    let mut app = App::new(Instant::from_millis(0), seed, Settings::DEFAULT);

    let mut display = SimulatorDisplay::<Rgb565>::new(layout::SCREEN_SIZE);
    let settings = OutputSettingsBuilder::new().scale(2).build();
    let mut window = Window::new("Wade", &settings);
    // The loop decides when to sleep; don't let the window throttle updates.
    window.set_max_fps(1_000);

    let Ok(()) = render::draw(&app.view(), &mut display);
    window.update(&display);

    if let Some(recording) = &recording {
        return replay_loop(
            recording,
            args.replay.as_ref().expect("replay path"),
            &clock,
            audio.as_ref(),
            &mut app,
            &mut display,
            &mut window,
        );
    }

    let mut writer = args
        .record
        .as_ref()
        .map(|path| File::create(path).map(BufWriter::new))
        .transpose()?;
    if let Some(writer) = writer.as_mut() {
        write!(writer, "{}", Recording::new(seed))?;
        writer.flush()?;
    }
    live_loop(
        &clock,
        audio.as_ref(),
        &mut app,
        &mut display,
        &mut window,
        &mut writer,
    )
}

fn live_loop(
    clock: &Clock,
    audio: Option<&Audio>,
    app: &mut App,
    display: &mut SimulatorDisplay<Rgb565>,
    window: &mut Window,
    writer: &mut Option<BufWriter<File>>,
) -> Result<(), Box<dyn Error>> {
    let mut mouse_down = false;
    loop {
        let mut redraw = false;

        for sim_event in window.events() {
            let now = clock.now();
            if sim_event == SimulatorEvent::Quit {
                if let Some(writer) = writer.as_mut() {
                    writer.flush()?;
                }
                return Ok(());
            }
            let event = input_event(sim_event, now, &mut mouse_down);
            if let Some(event) = event {
                redraw |= carry_out(&app.handle(event), audio);
                if let Some(writer) = writer.as_mut() {
                    record_event(writer, event, app)?;
                    writer.flush()?;
                }
            }
        }

        let now = clock.now();
        if app.next_deadline().is_some_and(|d| d <= now) {
            redraw |= carry_out(&app.handle(Event::deadline(now)), audio);
        }

        if redraw {
            let Ok(()) = render::draw(&app.view(), display);
            window.update(display);
        }

        let sleep = app
            .next_deadline()
            .map_or(POLL_INTERVAL, |d| clock.real_until(d).min(POLL_INTERVAL));
        std::thread::sleep(sleep);
    }
}

/// Carry out the core's effects and return whether to redraw. Nothing here
/// waits: the chime plays on the audio thread (D17).
fn carry_out(output: &Output, audio: Option<&Audio>) -> bool {
    for effect in &output.effects {
        match effect {
            Effect::Chime => {
                if let Some(audio) = audio {
                    audio.chime();
                }
            }
            Effect::SaveSettings(_) => {}
        }
    }
    output.redraw
}

fn input_event(sim_event: SimulatorEvent, now: Instant, mouse_down: &mut bool) -> Option<Event> {
    // The simulator reports points in display coordinates (it undoes the scale).
    match sim_event {
        SimulatorEvent::KeyDown {
            keycode,
            repeat: false,
            ..
        } => key(keycode).map(|key| Event::key(now, key)),
        SimulatorEvent::MouseButtonDown {
            mouse_btn: MouseButton::Left,
            point,
        } => {
            *mouse_down = true;
            Some(Event::touch(now, TouchPhase::Down, point))
        }
        SimulatorEvent::MouseMove { point } if *mouse_down => {
            Some(Event::touch(now, TouchPhase::Move, point))
        }
        SimulatorEvent::MouseButtonUp {
            mouse_btn: MouseButton::Left,
            point,
        } if *mouse_down => {
            *mouse_down = false;
            Some(Event::touch(now, TouchPhase::Up, point))
        }
        _ => None,
    }
}

fn record_event(writer: &mut impl Write, event: Event, app: &App) -> io::Result<()> {
    let input = match event.kind {
        EventKind::Touch(touch) => Input::Touch(touch),
        EventKind::Key(key) => Input::Key(key),
        EventKind::Deadline => return Ok(()),
    };
    writeln!(
        writer,
        "{}",
        Entry {
            at: event.at,
            input,
            hash: state_hash(app),
        }
    )
}

fn replay_loop(
    recording: &Recording,
    path: &Path,
    clock: &Clock,
    audio: Option<&Audio>,
    app: &mut App,
    display: &mut SimulatorDisplay<Rgb565>,
    window: &mut Window,
) -> Result<(), Box<dyn Error>> {
    for (index, entry) in recording.entries.iter().enumerate() {
        loop {
            if window
                .events()
                .any(|event| matches!(event, SimulatorEvent::Quit))
            {
                return Err("replay stopped before the last entry".into());
            }
            let now = clock.now();
            if let Some(deadline) = app
                .next_deadline()
                .filter(|deadline| *deadline <= entry.at && *deadline <= now)
            {
                if carry_out(&app.handle(Event::deadline(deadline)), audio) {
                    let Ok(()) = render::draw(&app.view(), display);
                    window.update(display);
                }
            } else if entry.at <= now {
                let output = app.handle(Event {
                    at: entry.at,
                    kind: entry.input.into(),
                });
                if let Some(mismatch) = entry.mismatch(index, state_hash(app)) {
                    return Err(format!("{}: {mismatch}", path.display()).into());
                }
                if carry_out(&output, audio) {
                    let Ok(()) = render::draw(&app.view(), display);
                    window.update(display);
                }
                break;
            } else {
                let next = app.next_deadline().map_or(entry.at, |d| d.min(entry.at));
                std::thread::sleep(clock.real_until(next).min(POLL_INTERVAL));
            }
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use embedded_graphics::geometry::Point;
    use embedded_graphics_simulator::sdl2::Mod;

    use super::*;

    #[test]
    fn captured_session_replays_and_omits_deadlines() {
        let mut app = App::new(Instant::from_millis(0), 42, Settings::DEFAULT);
        let mut output = Recording::new(42).to_string().into_bytes();
        let deadline = Event::deadline(Instant::from_millis(500));
        let _ = app.handle(deadline);
        record_event(&mut output, deadline, &app).unwrap();

        let press = |keycode| SimulatorEvent::KeyDown {
            keycode,
            keymod: Mod::NOMOD,
            repeat: false,
        };
        let down = |point| SimulatorEvent::MouseButtonDown {
            mouse_btn: MouseButton::Left,
            point,
        };
        let up = |point| SimulatorEvent::MouseButtonUp {
            mouse_btn: MouseButton::Left,
            point,
        };
        let samples = [
            (1_000, down(Point::new(160, 110))),
            (
                1_030,
                SimulatorEvent::MouseMove {
                    point: Point::new(162, 112),
                },
            ),
            (1_080, up(Point::new(162, 112))),
            (2_000, press(Keycode::NUM_3)),
            (2_500, press(Keycode::P)),
            (3_000, press(Keycode::Z)),
            (4_000, down(Point::new(160, 110))),
            (4_050, up(Point::new(160, 110))),
        ];
        let mut mouse_down = false;
        for (ms, sim_event) in samples {
            let event = input_event(sim_event, Instant::from_millis(ms), &mut mouse_down).unwrap();
            let _ = app.handle(event);
            record_event(&mut output, event, &app).unwrap();
        }
        let recording: Recording = String::from_utf8(output).unwrap().parse().unwrap();
        assert_eq!(recording.entries.len(), samples.len());
        let replay = recording.replay().unwrap();
        let wade_core::View::Buddy(buddy) = app.view() else {
            panic!("expected the Buddy screen");
        };
        assert_eq!(replay.harness.buddy().eye_style, buddy.eye_style);
    }
}
