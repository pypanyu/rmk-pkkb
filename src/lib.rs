//! Toykit v2 dongle firmware library crate.
//!
//! Exposes the custom OLED renderer so it can be referenced from `keyboard.toml`
//! as `renderer = "toykit_dongle::BongoCatRenderer"`. The three firmware
//! binaries (`central`, `peripheral`, `peripheral2`) are thin config-driven
//! entry points that depend on this lib.

#![no_std]

pub mod bongocat;
pub use bongocat::BongoCatRenderer;
