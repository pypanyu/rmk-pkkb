//! Dongle firmware: the split central (USB to host, BLE to the two halves).
//! The `#[rmk_central]` macro reads `keyboard.toml` and generates the full
//! keyboard + BLE + USB + display setup, including the bongocat OLED.

#![no_main]
#![no_std]

use rmk::macros::rmk_central;

#[rmk_central]
mod keyboard_central {}
