#![no_main]
#![no_std]

// 左半固件：分体中央端（split central）
// 硬件配置全部来自 keyboard.toml 的 [split.central] 段
use rmk::macros::rmk_central;

#[rmk_central]
mod keyboard_central {}
