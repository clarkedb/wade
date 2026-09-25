//! Wade's platform-independent core.
//!
//! All behavior lives here. The crate is `no_std` and does not use `alloc`: time
//! and randomness arrive as arguments, and side effects leave as return values.
//! See `docs/architecture.md`.

#![no_std]
#![warn(clippy::std_instead_of_core)]

// std is linked only for tests and the harness. Keeping the crate `no_std` means the
// std prelude never leaks into core modules; the harness imports what it uses.
#[cfg(any(test, feature = "harness"))]
extern crate std;

pub mod app;
pub mod character;
pub mod event;
pub mod input;
pub mod layout;
pub mod render;
pub mod rng;
pub mod time;
pub mod timer;
pub mod view;

#[cfg(any(test, feature = "harness"))]
pub mod harness;

pub use app::{App, Effect, Output};
pub use event::{Digit, Event, EventKind, Key, Touch, TouchPhase};
pub use time::{Duration, Instant};
pub use view::View;
