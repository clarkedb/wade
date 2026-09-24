//! Recordings: a seed plus every input event with the state hash after it
//! (docs/testing.md#recording-and-replay).
//!
//! ```text
//! wade-events 1
//! seed 8127364512
//! 1000 down 160 110 #3f9a1c02
//! 1080 up 160 110 #b7e0442d
//! ```
//!
//! `Deadline` events are not recorded; replay regenerates them from `next_deadline`.

use std::fmt;

use crate::event::Touch;
use crate::time::Instant;

use super::Harness;

/// The newest header version this code writes. The parser accepts every older one.
pub const VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recording {
    pub seed: u64,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub at: Instant,
    pub touch: Touch,
    /// State hash after the event was handled.
    pub hash: u32,
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

impl std::error::Error for ParseError {}

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

impl Recording {
    pub fn new(seed: u64) -> Self {
        Recording {
            seed,
            entries: Vec::new(),
        }
    }

    pub fn parse(_text: &str) -> Result<Recording, ParseError> {
        todo!("M1: parse the wade-events format")
    }

    pub fn write(&self, _out: &mut impl fmt::Write) -> fmt::Result {
        todo!("M1: write the wade-events format")
    }

    /// Run the recording through a fresh harness, checking every state hash.
    /// Returns the harness in its final state for further assertions.
    pub fn replay(&self) -> Result<Harness, Mismatch> {
        todo!("M1: replay each entry with Harness::touch and compare hashes")
    }
}
