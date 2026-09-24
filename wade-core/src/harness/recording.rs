//! Recordings: a seed plus every input event with the state hash after it
//! (docs/testing.md#recording-and-replay).
//!
//! ```text
//! wade-events 1
//! seed 8127364512
//! 1000 down 160 110 #3f9a1c02
//! 1080 up 160 110 #b7e0442d
//! 2400 key 3 #5d21a7c4
//! ```
//!
//! `Deadline` events are not recorded; replay regenerates them from `next_deadline`.
//! Parse with `text.parse::<Recording>()`; write with `Display`.

use core::fmt;
use core::str::FromStr;
use std::string::String;
use std::vec::Vec;

use crate::event::{EventKind, Key, Touch};
use crate::time::Instant;

use super::Harness;

/// The newest header version this code writes. The parser accepts every older one.
pub const VERSION: u32 = 1;

/// File extension for recordings in `wade-core/tests/recordings/`.
pub const EXTENSION: &str = "events";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recording {
    pub seed: u64,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub at: Instant,
    pub input: Input,
    /// State hash after the event was handled.
    pub hash: u32,
}

/// A recorded input: every event kind except `Deadline`, which replay regenerates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Touch(Touch),
    Key(Key),
}

impl From<Input> for EventKind {
    fn from(input: Input) -> EventKind {
        match input {
            Input::Touch(touch) => EventKind::Touch(touch),
            Input::Key(key) => EventKind::Key(key),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl core::error::Error for ParseError {}

/// The first entry whose recomputed hash differs from the recorded one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mismatch {
    pub index: usize,
    pub at: Instant,
    pub expected: u32,
    pub actual: u32,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "entry {} at {} ms: expected #{:08x}, got #{:08x}",
            self.index,
            self.at.as_millis(),
            self.expected,
            self.actual
        )
    }
}

impl core::error::Error for Mismatch {}

/// The result of running a recording: the harness in its final state, and the
/// state hash recomputed after each entry.
#[derive(Debug)]
pub struct Replay {
    pub harness: Harness,
    pub hashes: Vec<u32>,
}

impl Recording {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self {
            seed,
            entries: Vec::new(),
        }
    }

    /// Run every entry through a fresh harness, recomputing each state hash.
    /// Compare with [`Recording::first_mismatch`], or write the new hashes back
    /// for `UPDATE_RECORDINGS=1`.
    #[must_use]
    pub fn replay(&self) -> Replay {
        todo!("M1: replay each entry through the harness and collect state hashes")
    }

    /// The first entry whose recorded hash differs from `hashes`.
    #[must_use]
    pub fn first_mismatch(&self, hashes: &[u32]) -> Option<Mismatch> {
        self.entries
            .iter()
            .zip(hashes)
            .enumerate()
            .find_map(|(index, (entry, &actual))| {
                (entry.hash != actual).then_some(Mismatch {
                    index,
                    at: entry.at,
                    expected: entry.hash,
                    actual,
                })
            })
    }
}

impl FromStr for Recording {
    type Err = ParseError;

    fn from_str(_text: &str) -> Result<Self, ParseError> {
        todo!("M1: parse the wade-events format, accepting every version up to VERSION")
    }
}

impl fmt::Display for Recording {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("M1: write the wade-events format at VERSION")
    }
}
