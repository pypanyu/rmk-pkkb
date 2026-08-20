//! Left-half firmware: split peripheral 0 (col_offset 0).

#![no_main]
#![no_std]

use rmk::macros::rmk_peripheral;

#[rmk_peripheral(id = 0)]
mod keyboard_peripheral {}
