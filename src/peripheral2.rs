//! Right-half firmware: split peripheral 1 (col_offset 5).

#![no_main]
#![no_std]

use rmk::macros::rmk_peripheral;

#[rmk_peripheral(id = 1)]
mod keyboard_peripheral {}
