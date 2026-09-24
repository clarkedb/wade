//! Replays every recording in tests/recordings/ and checks every state hash
//! (docs/testing.md#recording-and-replay).

use std::path::PathBuf;

use wade_core::harness::recording::{FILE_SUFFIX, Input, Recording};
use wade_core::view::EyeStyle;
use wade_core::{Key, TouchPhase};

#[test]
fn recordings_replay_with_matching_hashes() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recordings");
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
        let text = std::fs::read_to_string(&path).expect("read recording");
        let mut recording: Recording = text
            .parse()
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if std::env::var("UPDATE_RECORDINGS").is_ok_and(|value| value == "1") {
            recording.refresh_hashes();
            std::fs::write(&path, recording.to_string()).expect("write recording");
        }
        let replay = recording
            .replay()
            .unwrap_or_else(|m| panic!("{}: {m}", path.display()));
        if path
            .file_name()
            .is_some_and(|name| name == "desktop-m1.events.wade")
        {
            assert!(recording
                .entries
                .iter()
                .any(|entry| matches!(entry.input, Input::Touch(touch) if touch.phase == TouchPhase::Down)));
            assert!(recording.entries.iter().any(
                |entry| matches!(entry.input, Input::Touch(touch) if touch.phase == TouchPhase::Move)
            ));
            assert!(recording.entries.iter().any(
                |entry| matches!(entry.input, Input::Touch(touch) if touch.phase == TouchPhase::Up)
            ));
            for key in [
                Key::Digit(wade_core::Digit::new(3).unwrap()),
                Key::Z,
                Key::P,
            ] {
                assert!(
                    recording
                        .entries
                        .iter()
                        .any(|entry| entry.input == Input::Key(key))
                );
            }
            assert_eq!(replay.harness.buddy().eye_style, EyeStyle::Plain);
            assert!(!replay.harness.buddy().asleep);
        }
    }
}
