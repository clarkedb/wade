//! Desktop simulator for Wade (docs/platforms.md#desktop-wade-desktop).
//!
//! Runs the platform loop from docs/architecture.md#core-api against an
//! `embedded-graphics-simulator` window.

mod audio;
mod clock;
mod screen;
mod storage;

use std::error::Error;
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorEvent, Window,
    sdl2::{Keycode, MouseButton},
};
use wade_core::harness::recording::{Entry, Input, Recording};
use wade_core::harness::state_hash;
use wade_core::timer::TimerPhase;
use wade_core::{
    App, BuildInfo, Digit, Effect, Event, EventKind, Instant, Key, Output, Settings, TouchPhase,
};

use audio::Audio;
use clock::Clock;
use screen::Screen;
use storage::{SettingsSaver, Storage};

/// Longest the loop sleeps before polling window events again.
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_REPLAY_BYTES: usize = 16 * 1024 * 1024;
const BUILD_INFO: BuildInfo = BuildInfo {
    version: env!("WADE_VERSION"),
    board: "Desktop",
    revision: env!("WADE_REVISION"),
    development: env!("WADE_DEVELOPMENT").as_bytes()[0] == b't',
    dirty: env!("WADE_DIRTY").as_bytes()[0] == b't',
};

#[derive(Parser, Debug)]
#[command(version = env!("WADE_BUILD_VERSION"), about = "Wade desktop simulator")]
struct Args {
    /// Fixed random seed. Defaults to OS entropy.
    #[arg(long, conflicts_with = "replay")]
    seed: Option<u64>,

    /// Write the seed, the starting settings, and every input event to a recording.
    #[arg(long, value_name = "FILE", conflicts_with = "replay")]
    record: Option<PathBuf>,

    /// Play a recording back in real time instead of reading the mouse.
    #[arg(long, value_name = "FILE")]
    replay: Option<PathBuf>,

    /// Run the clock this many times faster (0.01 to 1000).
    #[arg(long, value_name = "X", default_value_t = 1.0, value_parser = parse_time_scale)]
    time_scale: f64,

    /// Draw each frame in strips of this many rows, as a low-memory board would (1 to 240).
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u16).range(1..=240))]
    band_rows: Option<u16>,
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
    eprintln!(
        "Wade {} / {} / build {}",
        env!("WADE_BUILD_VERSION"),
        BUILD_INFO.board,
        BUILD_INFO.revision
    );
    let recording = if let Some(path) = &args.replay {
        let text = read_limited(File::open(path)?, MAX_REPLAY_BYTES)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let recording: Recording = text
            .parse()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        recording
            .replay()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Some(recording)
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

    // A replay starts with the settings it recorded, and saves nothing.
    let storage = recording.is_none().then(Storage::in_config_dir).flatten();
    if recording.is_none() && storage.is_none() {
        eprintln!("no config directory, so settings are not kept");
    }
    let settings = match (&recording, &storage) {
        (Some(recording), _) => recording.settings,
        (None, Some(storage)) => storage.load(),
        (None, None) => Settings::DEFAULT,
    };

    let audio = Audio::open();
    let clock = Clock::new(args.time_scale);
    let mut app = App::new(Instant::from_millis(0), seed, settings).with_build_info(BUILD_INFO);

    let mut screen = Screen::new(args.band_rows.map(usize::from), settings.brightness());
    let output_settings = OutputSettingsBuilder::new().scale(2).build();
    let mut window = Window::new("Wade", &output_settings);
    // The loop decides when to sleep; don't let the window throttle updates.
    window.set_max_fps(1_000);

    screen.show(&app.view(), &mut window);

    if let Some(recording) = &recording {
        return replay_loop(
            recording,
            args.replay.as_ref().expect("replay path"),
            &clock,
            audio.as_ref(),
            &mut app,
            &mut screen,
            &mut window,
        );
    }

    let mut writer = args
        .record
        .as_ref()
        .map(|path| File::create(path).map(BufWriter::new))
        .transpose()?;
    if let Some(writer) = writer.as_mut() {
        write!(writer, "{}", Recording::new(seed, settings))?;
        writer.flush()?;
    }
    let mut saver = storage.as_ref().map(SettingsSaver::new);
    live_loop(
        &clock,
        audio.as_ref(),
        &mut saver,
        &mut app,
        &mut screen,
        &mut window,
        &mut writer,
    )
}

fn read_limited(reader: impl Read, max_bytes: usize) -> io::Result<String> {
    let mut bytes = Vec::new();
    reader.take(max_bytes as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("recording exceeds {max_bytes} bytes"),
        ));
    }
    String::from_utf8(bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn live_loop(
    clock: &Clock,
    audio: Option<&Audio>,
    saver: &mut Option<SettingsSaver<'_>>,
    app: &mut App,
    screen: &mut Screen,
    window: &mut Window,
    writer: &mut Option<BufWriter<File>>,
) -> Result<(), Box<dyn Error>> {
    let mut mouse_down = false;
    loop {
        let mut redraw = false;
        let mut skip_queued_touch = false;
        let events: Vec<_> = window.events().collect();
        for sim_event in events {
            let now = clock.now();
            if let Some((output, became_done)) = due_deadline(app, now) {
                redraw |= carry_out(&output, audio, saver.as_mut(), screen);
                if became_done {
                    screen.show(&app.view(), window);
                    redraw = false;
                    skip_queued_touch = true;
                }
            }
            if sim_event == SimulatorEvent::Quit {
                if let Some(saver) = saver.as_mut() {
                    saver.flush(app.unsaved_settings());
                }
                if let Some(writer) = writer.as_mut() {
                    writer.flush()?;
                }
                return Ok(());
            }
            let event = queued_input_event(sim_event, now, &mut mouse_down, skip_queued_touch);
            if let Some(event) = event {
                redraw |= carry_out(&app.handle(event), audio, saver.as_mut(), screen);
                if let Some(writer) = writer.as_mut() {
                    record_event(writer, event, app)?;
                    writer.flush()?;
                }
            }
        }

        let now = clock.now();
        if let Some((output, became_done)) = due_deadline(app, now) {
            redraw |= carry_out(&output, audio, saver.as_mut(), screen);
            if became_done {
                screen.show(&app.view(), window);
                redraw = false;
            }
        }

        if let Some(saver) = saver.as_mut() {
            saver.retry_if_due();
        }

        if redraw {
            screen.show(&app.view(), window);
        }

        let sleep = app
            .next_deadline()
            .map_or(POLL_INTERVAL, |d| clock.real_until(d).min(POLL_INTERVAL));
        std::thread::sleep(sleep);
    }
}

fn due_deadline(app: &mut App, now: Instant) -> Option<(Output, bool)> {
    app.next_deadline().filter(|&deadline| deadline <= now)?;
    let was_done = app.timer_state().phase() == TimerPhase::Done;
    let output = app.handle(Event::deadline(now));
    let became_done = !was_done && app.timer_state().phase() == TimerPhase::Done;
    Some((output, became_done))
}

fn queued_input_event(
    sim_event: SimulatorEvent,
    now: Instant,
    mouse_down: &mut bool,
    skip_touch: bool,
) -> Option<Event> {
    if skip_touch
        && matches!(
            sim_event,
            SimulatorEvent::MouseButtonDown { .. }
                | SimulatorEvent::MouseMove { .. }
                | SimulatorEvent::MouseButtonUp { .. }
        )
    {
        *mouse_down = false;
        return None;
    }
    input_event(sim_event, now, mouse_down)
}

/// Carry out the core's effects and return whether to redraw. Nothing here
/// waits long: the chime plays on the audio thread (D17), and a save writes
/// six bytes.
fn carry_out(
    output: &Output,
    audio: Option<&Audio>,
    mut saver: Option<&mut SettingsSaver<'_>>,
    screen: &mut Screen,
) -> bool {
    for &effect in &output.effects {
        match effect {
            Effect::Chime => {
                if let Some(audio) = audio {
                    audio.chime();
                }
            }
            Effect::SaveSettings(settings) => {
                if let Some(saver) = saver.as_mut() {
                    saver.save(settings);
                }
            }
            Effect::SetBrightness(brightness) => screen.set_brightness(brightness),
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
    screen: &mut Screen,
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
                if carry_out(&app.handle(Event::deadline(deadline)), audio, None, screen) {
                    screen.show(&app.view(), window);
                }
            } else if entry.at <= now {
                let output = app.handle(Event {
                    at: entry.at,
                    kind: entry.input.into(),
                });
                if let Some(mismatch) = entry.mismatch(index, state_hash(app)) {
                    return Err(format!("{}: {mismatch}", path.display()).into());
                }
                if carry_out(&output, audio, None, screen) {
                    screen.show(&app.view(), window);
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
    use std::io::Cursor;
    use wade_core::layout;

    use super::*;

    #[test]
    fn late_completion_displays_done_before_queued_touches_can_dismiss_it() {
        use wade_core::timer::TimerState;

        let settings = Settings::DEFAULT
            .with_timer(Duration::from_secs(60))
            .unwrap();
        let mut app = App::new(Instant::from_millis(0), 42, settings);
        let tap = |app: &mut App, at, point| {
            let _ = app.handle(Event::touch(at, TouchPhase::Down, point));
            let _ = app.handle(Event::touch(at, TouchPhase::Up, point));
        };
        let at = Instant::from_millis(1_000);
        tap(&mut app, at, layout::APPS.center());
        tap(&mut app, at, layout::TILES[0].1.center());
        tap(
            &mut app,
            Instant::from_millis(2_000),
            layout::TIMER_ROW[1].center(),
        );
        tap(&mut app, Instant::from_millis(3_000), layout::BACK.center());
        tap(&mut app, Instant::from_millis(3_000), layout::BACK.center());
        assert_eq!(app.screen(), wade_core::app::Screen::Buddy);
        assert!(matches!(app.timer_state(), TimerState::Running { .. }));

        let late = Instant::from_millis(62_600);
        let (_, became_done) = due_deadline(&mut app, late).unwrap();
        assert!(became_done);
        let point = layout::TIMER_ROW[1].center();
        let mut mouse_down = false;
        for sim_event in [
            SimulatorEvent::MouseButtonDown {
                mouse_btn: MouseButton::Left,
                point,
            },
            SimulatorEvent::MouseButtonUp {
                mouse_btn: MouseButton::Left,
                point,
            },
        ] {
            assert_eq!(
                queued_input_event(sim_event, late, &mut mouse_down, became_done),
                None
            );
        }
        assert!(matches!(app.timer_state(), TimerState::Done { .. }));
        assert_eq!(app.screen(), wade_core::app::Screen::Timer);

        tap(&mut app, Instant::from_millis(63_200), point);
        assert!(matches!(app.timer_state(), TimerState::Ready { .. }));
        assert_eq!(app.screen(), wade_core::app::Screen::Buddy);
    }

    #[test]
    fn keyboard_input_maps_keys_and_ignores_repeats() {
        let at = Instant::from_millis(123);
        let mut mouse_down = false;
        let press = |keycode, repeat| SimulatorEvent::KeyDown {
            keycode,
            keymod: Mod::NOMOD,
            repeat,
        };
        for (keycode, expected) in [
            (Keycode::NUM_0, Key::Digit(Digit::new(0).unwrap())),
            (Keycode::NUM_3, Key::Digit(Digit::new(3).unwrap())),
            (Keycode::NUM_9, Key::Digit(Digit::new(9).unwrap())),
            (Keycode::Z, Key::Z),
            (Keycode::P, Key::P),
        ] {
            assert_eq!(
                input_event(press(keycode, false), at, &mut mouse_down),
                Some(Event::key(at, expected))
            );
            assert_eq!(input_event(press(keycode, true), at, &mut mouse_down), None);
        }
        assert_eq!(
            input_event(press(Keycode::A, false), at, &mut mouse_down),
            None
        );
    }

    #[test]
    fn mouse_input_tracks_only_a_left_button_press() {
        let at = Instant::from_millis(123);
        let mut mouse_down = false;
        let down = |mouse_btn, point| SimulatorEvent::MouseButtonDown { mouse_btn, point };
        let up = |mouse_btn, point| SimulatorEvent::MouseButtonUp { mouse_btn, point };
        let start = Point::new(12, 34);
        let moved = Point::new(56, 78);

        assert_eq!(
            input_event(up(MouseButton::Left, start), at, &mut mouse_down),
            None
        );
        assert_eq!(
            input_event(
                SimulatorEvent::MouseMove { point: moved },
                at,
                &mut mouse_down
            ),
            None
        );
        assert_eq!(
            input_event(down(MouseButton::Right, start), at, &mut mouse_down),
            None
        );
        assert_eq!(
            input_event(down(MouseButton::Left, start), at, &mut mouse_down),
            Some(Event::touch(at, TouchPhase::Down, start))
        );
        assert_eq!(
            input_event(
                SimulatorEvent::MouseMove { point: moved },
                at,
                &mut mouse_down
            ),
            Some(Event::touch(at, TouchPhase::Move, moved))
        );
        assert_eq!(
            input_event(up(MouseButton::Right, moved), at, &mut mouse_down),
            None
        );
        assert_eq!(
            input_event(up(MouseButton::Left, moved), at, &mut mouse_down),
            Some(Event::touch(at, TouchPhase::Up, moved))
        );
        assert_eq!(
            input_event(up(MouseButton::Left, moved), at, &mut mouse_down),
            None
        );
    }

    #[test]
    fn captured_session_replays_and_omits_deadlines() {
        let mut app = App::new(Instant::from_millis(0), 42, Settings::DEFAULT);
        let mut output = Recording::new(42, Settings::DEFAULT)
            .to_string()
            .into_bytes();
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

    #[test]
    fn replay_file_limit_accepts_the_boundary_and_rejects_one_byte_more() {
        assert_eq!(read_limited(Cursor::new(b"hello"), 5).unwrap(), "hello");
        let error = read_limited(Cursor::new(b"hello"), 4).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("exceeds 4 bytes"));
    }
}
