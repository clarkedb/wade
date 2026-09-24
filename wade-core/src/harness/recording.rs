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

use embedded_graphics::geometry::Point;

use crate::event::{Digit, Event, EventKind, Key, Touch, TouchPhase};
use crate::time::Instant;

use super::Harness;

/// The newest header version this code writes. The parser accepts every older one.
pub const VERSION: u32 = 1;

/// Filename suffix for recordings in `wade-core/tests/recordings/`.
pub const FILE_SUFFIX: &str = ".events.wade";

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

impl Entry {
    #[must_use]
    pub fn mismatch(self, index: usize, actual: u32) -> Option<Mismatch> {
        (self.hash != actual).then_some(Mismatch {
            index,
            at: self.at,
            expected: self.hash,
            actual,
        })
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.at.as_millis())?;
        match self.input {
            Input::Touch(Touch { phase, point }) => {
                let name = match phase {
                    TouchPhase::Down => "down",
                    TouchPhase::Move => "move",
                    TouchPhase::Up => "up",
                };
                write!(f, "{name} {} {}", point.x, point.y)?;
            }
            Input::Key(key) => match key {
                Key::Digit(digit) => write!(f, "key {}", digit.get())?,
                Key::Z => f.write_str("key z")?,
                Key::P => f.write_str("key p")?,
            },
        }
        write!(f, " #{:08x}", self.hash)
    }
}

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

    /// Run every entry through a fresh harness and stop at the first mismatch.
    ///
    /// # Errors
    ///
    /// Returns the first entry whose recorded hash differs from the replayed state.
    ///
    /// # Panics
    ///
    /// Panics if entries are not ordered by timestamp.
    pub fn replay(&self) -> Result<Replay, Mismatch> {
        self.run(true)
    }

    /// Recompute hashes for an intentional behavior change.
    ///
    /// # Panics
    ///
    /// Panics if entries are not ordered by timestamp.
    pub fn refresh_hashes(&mut self) {
        let replay = self.run(false).expect("unchecked replay cannot mismatch");
        for (entry, hash) in self.entries.iter_mut().zip(replay.hashes) {
            entry.hash = hash;
        }
    }

    fn run(&self, check: bool) -> Result<Replay, Mismatch> {
        let mut harness = Harness::new(self.seed);
        let mut hashes = Vec::with_capacity(self.entries.len());
        for (index, entry) in self.entries.iter().enumerate() {
            harness.run_until(entry.at);
            harness.handle(Event {
                at: entry.at,
                kind: entry.input.into(),
            });
            let hash = harness.state_hash();
            if check && let Some(mismatch) = entry.mismatch(index, hash) {
                return Err(mismatch);
            }
            hashes.push(hash);
        }
        Ok(Replay { harness, hashes })
    }

    /// The first entry whose recorded hash differs from `hashes`.
    ///
    /// # Panics
    ///
    /// Panics if there is not one recomputed hash per entry.
    #[must_use]
    pub fn first_mismatch(&self, hashes: &[u32]) -> Option<Mismatch> {
        assert_eq!(self.entries.len(), hashes.len(), "missing replay hashes");
        self.entries
            .iter()
            .zip(hashes)
            .enumerate()
            .find_map(|(index, (entry, &actual))| entry.mismatch(index, actual))
    }
}

impl FromStr for Recording {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, ParseError> {
        let mut lines = text.lines().enumerate();
        let header = lines.next().ok_or_else(|| error(1, "missing header"))?;
        let mut fields = header.1.split_whitespace();
        if fields.next() != Some("wade-events") {
            return Err(error(1, "expected wade-events header"));
        }
        let version: u32 = parse_field(&mut fields, 1, "version")?;
        if version == 0 || version > VERSION || fields.next().is_some() {
            return Err(error(1, "unsupported recording version"));
        }

        let seed_line = lines.next().ok_or_else(|| error(2, "missing seed"))?;
        let mut fields = seed_line.1.split_whitespace();
        if fields.next() != Some("seed") {
            return Err(error(2, "expected seed"));
        }
        let seed = parse_field(&mut fields, 2, "seed")?;
        if fields.next().is_some() {
            return Err(error(2, "unexpected field after seed"));
        }

        let mut recording = Recording::new(seed);
        for (line_index, line) in lines {
            let line_number = line_index + 1;
            let mut fields = line.split_whitespace();
            let at = Instant::from_millis(parse_field(&mut fields, line_number, "timestamp")?);
            if recording.entries.last().is_some_and(|entry| at < entry.at) {
                return Err(error(line_number, "timestamp precedes previous entry"));
            }
            let kind = fields
                .next()
                .ok_or_else(|| error(line_number, "missing input kind"))?;
            let input = match kind {
                "down" | "move" | "up" => {
                    let phase = match kind {
                        "down" => TouchPhase::Down,
                        "move" => TouchPhase::Move,
                        _ => TouchPhase::Up,
                    };
                    let x = parse_field(&mut fields, line_number, "x")?;
                    let y = parse_field(&mut fields, line_number, "y")?;
                    Input::Touch(Touch {
                        phase,
                        point: Point::new(x, y),
                    })
                }
                "key" => {
                    let name = fields
                        .next()
                        .ok_or_else(|| error(line_number, "missing key"))?;
                    let key = match name {
                        "z" => Key::Z,
                        "p" => Key::P,
                        _ => {
                            let digit = name
                                .parse::<u8>()
                                .ok()
                                .filter(|_| name.len() == 1)
                                .and_then(Digit::new)
                                .ok_or_else(|| error(line_number, "invalid key"))?;
                            Key::Digit(digit)
                        }
                    };
                    Input::Key(key)
                }
                _ => return Err(error(line_number, "invalid input kind")),
            };
            let hash_text = fields
                .next()
                .ok_or_else(|| error(line_number, "missing state hash"))?;
            let hash = hash_text
                .strip_prefix('#')
                .filter(|hex| hex.len() == 8)
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .ok_or_else(|| error(line_number, "invalid state hash"))?;
            if fields.next().is_some() {
                return Err(error(line_number, "unexpected field after state hash"));
            }
            recording.entries.push(Entry { at, input, hash });
        }
        Ok(recording)
    }
}

impl fmt::Display for Recording {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "wade-events {VERSION}")?;
        writeln!(f, "seed {}", self.seed)?;
        for entry in &self.entries {
            writeln!(f, "{entry}")?;
        }
        Ok(())
    }
}

fn error(line: usize, message: &str) -> ParseError {
    ParseError {
        line,
        message: String::from(message),
    }
}

fn parse_field<T: FromStr>(
    fields: &mut core::str::SplitWhitespace<'_>,
    line: usize,
    name: &str,
) -> Result<T, ParseError> {
    fields
        .next()
        .and_then(|field| field.parse().ok())
        .ok_or_else(|| error(line, &std::format!("missing or invalid {name}")))
}

#[cfg(test)]
mod tests {
    use std::string::ToString;

    use super::*;

    #[test]
    fn parses_and_writes_each_input_kind() {
        let source = "wade-events 1\nseed 42\n0 down -1 240 #0123abcd\n0 move 2 3 #89abcdef\n10 up 2 3 #ffffffff\n10 key 9 #00000000\n20 key z #00000001\n30 key p #00000002\n";
        let recording: Recording = source.parse().unwrap();
        assert_eq!(recording.seed, 42);
        assert_eq!(recording.entries.len(), 6);
        assert_eq!(recording.to_string(), source);
    }

    #[test]
    fn reports_the_bad_line() {
        for (source, line) in [
            ("", 1),
            ("wade-events 2\nseed 1\n", 1),
            ("wade-events 1\nseed nope\n", 2),
            ("wade-events 1\nseed 1\n0 deadline #00000000\n", 3),
            ("wade-events 1\nseed 1\n0 key 10 #00000000\n", 3),
            (
                "wade-events 1\nseed 1\n10 key p #00000000\n9 key z #00000000\n",
                4,
            ),
        ] {
            assert_eq!(source.parse::<Recording>().unwrap_err().line, line);
        }
    }

    #[test]
    fn replay_reports_first_mismatch_with_timestamp() {
        let mut recording = Recording::new(42);
        for (at, key) in [(1_000, Key::P), (2_000, Key::Z), (3_000, Key::P)] {
            recording.entries.push(Entry {
                at: Instant::from_millis(at),
                input: Input::Key(key),
                hash: 0,
            });
        }
        recording.refresh_hashes();
        assert_ne!(recording.entries[0].hash, Harness::new(42).state_hash());
        assert_ne!(recording.entries[0].hash, recording.entries[2].hash);
        assert!(recording.replay().is_ok());
        recording.entries[1].hash ^= 1;
        recording.entries[2].hash ^= 1;
        let mismatch = recording.replay().unwrap_err();
        assert_eq!(mismatch.index, 1);
        assert_eq!(mismatch.at, Instant::from_millis(2_000));
        assert!(mismatch.to_string().contains("entry 1 at 2000 ms"));
    }
}
