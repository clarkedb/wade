//! Device logic shared by Wade's firmware crates: the parts that are neither
//! Wade's behavior (`wade-core`) nor hardware access (the firmware itself).
//! `no_std` and allocation-free, so the firmware can link it, and tested on
//! the host. See docs/architecture.md#crates.

#![no_std]

pub mod settings_store;
pub mod xpt2046;
