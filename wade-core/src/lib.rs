//! Wade's platform-independent core.
//!
//! All behavior lives here. The crate is `no_std` and does not use `alloc`: time
//! and randomness arrive as arguments, and side effects leave as return values.
//! See `docs/architecture.md`.

#![cfg_attr(not(any(test, feature = "harness")), no_std)]

pub mod app;
pub mod character;
pub mod event;
pub mod input;
pub mod layout;
pub mod render;
pub mod rng;
pub mod time;
pub mod view;

#[cfg(any(test, feature = "harness"))]
pub mod harness;

pub use app::{App, Effect, Output};
pub use event::{Event, EventKind, Touch, TouchPhase};
pub use time::{Duration, Instant};
pub use view::View;
