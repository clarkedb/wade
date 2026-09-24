//! Replays every recording in tests/recordings/ and checks every state hash
//! (docs/testing.md#recording-and-replay).

use std::path::PathBuf;

use wade_core::harness::recording::Recording;

/// Recording files end in this extension.
const EXTENSION: &str = "events";

#[test]
fn recordings_replay_with_matching_hashes() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recordings");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .expect("read tests/recordings")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == EXTENSION))
        .collect();
    paths.sort();

    for path in paths {
        let text = std::fs::read_to_string(&path).expect("read recording");
        let recording =
            Recording::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if let Err(m) = recording.replay() {
            panic!("{}: {m}", path.display());
        }
    }
}
