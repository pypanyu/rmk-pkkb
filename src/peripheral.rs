#![no_main]
#![no_std]

// 右半固件：分体外设端（split peripheral，id = 0 对应第一个 [[split.peripheral]]）
// 硬件配置全部来自 keyboard.toml 的 [[split.peripheral]] 段
use rmk::macros::rmk_peripheral;

#[rmk_peripheral(id = 0)]
mod keyboard_peripheral {}
