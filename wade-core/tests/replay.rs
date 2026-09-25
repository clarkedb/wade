//! Replays every recording in tests/recordings/ and checks every state hash,
//! then checks what particular recordings are for
//! (docs/testing.md#recording-and-replay).

use std::path::{Path, PathBuf};

use wade_core::harness::recording::{FILE_SUFFIX, Input, Recording, Replay};
use wade_core::view::EyeStyle;
use wade_core::{Digit, Key, TouchPhase};

fn recordings_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recordings")
}

/// Parse the recording at `path`. With `UPDATE_RECORDINGS=1`, its hashes are
/// recomputed first, for an intentional behavior change.
fn load(path: &Path) -> Recording {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut recording: Recording = text
        .parse()
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    if std::env::var("UPDATE_RECORDINGS").is_ok_and(|value| value == "1") {
        recording.refresh_hashes();
    }
    recording
}

/// Replay the recording `name` in tests/recordings/, checking every hash.
fn replay(name: &str) -> (Recording, Replay) {
    let path = recordings_dir().join(format!("{name}{FILE_SUFFIX}"));
    let recording = load(&path);
    let replay = recording
        .replay()
        .unwrap_or_else(|m| panic!("{}: {m}", path.display()));
    (recording, replay)
}

#[test]
fn every_recording_replays_with_matching_hashes() {
    let dir = recordings_dir();
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .expect("read tests/recordings")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| {
            p.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(FILE_SUFFIX))
        })
        .collect();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "no *{FILE_SUFFIX} recordings in {}",
        dir.display()
    );
    for path in paths {
        let recording = load(&path);
        if std::env::var("UPDATE_RECORDINGS").is_ok_and(|value| value == "1") {
            std::fs::write(&path, recording.to_string()).expect("write recording");
        }
        if let Err(mismatch) = recording.replay() {
            panic!("{}: {mismatch}", path.display());
        }
    }
}

#[test]
fn the_m1_session_covers_every_input_and_ends_awake_with_plain_eyes() {
    let (recording, replay) = replay("desktop-m1");
    let has = |matches: &dyn Fn(Input) -> bool| recording.entries.iter().any(|e| matches(e.input));
    for phase in [TouchPhase::Down, TouchPhase::Move, TouchPhase::Up] {
        assert!(
            has(&|input| matches!(input, Input::Touch(touch) if touch.phase == phase)),
            "no {phase:?}"
        );
    }
    let three = Key::Digit(Digit::new(3).expect("a digit"));
    for key in [three, Key::Z, Key::P] {
        assert!(has(&|input| input == Input::Key(key)), "no {key:?}");
    }
    assert_eq!(replay.harness.buddy().eye_style, EyeStyle::Plain);
    assert!(!replay.harness.buddy().asleep);
}
