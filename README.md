# Toykit v2 — BLE split keyboard + USB dongle with bongocat OLED

在原有 RMK 分体键盘基础上增加了一个 **USB dongle（加密狗）**：

- **dongle（central）**：插在电脑 USB 上，通过 BLE 连接左右两个半体，并在一块
  小 OLED 上显示 **bongocat**。dongle 本身没有物理按键，所有按键都来自两个半体。
- **left half（peripheral 0，col_offset 0）**：原来的 central 半体。
- **right half（peripheral 1，col_offset 5）**：原来的 peripheral 半体。

整块键盘仍是 `4 行 × 10 列`，keymap 与原工程完全一致。

## 代码结构（遵循 RMK 的 split dongle 示例）

```
keyboard.toml            # 配置：split.central= dongle + [split.central.display]
vial.json                # 原样保留（Vial 配置）
Cargo.toml               # rmk 依赖开启 display + ssd1306，并导出本 crate 的 renderer
build.rs / memory.x      # 链接脚本 + Vial 配置生成
.cargo/config.toml       # 交叉编译目标 + KEYBOARD_TOML_PATH
Makefile.toml            # 生成 .uf2 / .hex
rust-toolchain.toml      # 固定 1.96.0
src/
  lib.rs                 # 导出 BongoCatRenderer
  bongocat.rs            # 自定义 DisplayRenderer：画 bongocat 并随按键动画
  central.rs             # dongle 入口（#[rmk_central] 读取 keyboard.toml）
  peripheral.rs          # 左半体（#[rmk_peripheral(id = 0)]）
  peripheral2.rs         # 右半体（#[rmk_peripheral(id = 1)]）
```

核心是 `keyboard.toml` 里的这一段（dongle 的小屏 + bongocat）：

```toml
[split.central.display]
driver = "ssd1306"
size = "128x64"
rotation = 0
renderer = "toykit_dongle::BongoCatRenderer"
render_interval = 40
min_render_interval = 10
[split.central.display.protocol.i2c]
instance = "TWISPI0"
scl = "P0_17"
sda = "P0_20"
```

`renderer` 指向 `src/bongocat.rs` 里实现的 `BongoCatRenderer`
（`impl DisplayRenderer<BinaryColor>`）。它利用 `RenderContext` 的
`key_press_latch` / `key_pressed` 字段：每次新按键翻转一帧（敲鼓动画），
按住时定格在“爪子落下”，空闲时缓慢呼吸。

## 你需要确认/修改的地方

1. **OLED 的 I2C 引脚**：上面默认 `TWISPI0`、`P0_17`(SCL)/`P0_20`(SDA)，
   请改成你 dongle 板子上实际接 OLED 的引脚。屏幕分辨率若不是 `128x64`
   （如 `128x32`），改 `size` 并相应调整 `src/bongocat.rs` 里的坐标。
2. **dongle 无键矩阵**：central 设成 `rows = 0, cols = 0` + 空
   `direct_pin` 矩阵。如果某个 RMK 版本不允许 0×0 central 矩阵，
   把它改成 `1×1` 的 `direct_pin` 哑矩阵，用一个 dongle 上空闲的 GPIO 即可
   （不影响功能，只是多一个虚拟键）。
3. **两个半体的 `ble_addr`** 已保留原值，便于它们与 dongle 配对。
   dongle（central）的地址由 RMK 自动生成，两个半体会自动连上它。
4. **RMK 版本**：`display` / `ssd1306` 特性需要较新的 RMK（≥ 0.8 / `main`）。
   若 `cargo build` 报找不到该 feature，请把 `Cargo.toml` 里 `rmk` 依赖
   固定到一个带这两个特性的 tag/rev。

## 构建与烧录

需要：Rust + `rustup target add thumbv7em-none-eabihf`、`cargo install flip-link cargo-make cargo-binutils cargo-hex-to-uf2`，
以及 nRF52840 的 S140 softdevice。

```shell
# 三个固件
cargo build --release --bin central       # dongle
cargo build --release --bin peripheral    # 左半体
cargo build --release --bin peripheral2   # 右半体

# 生成 .uf2（nice!nano 等带 Adafruit bootloader 的板子）
cargo make uf2 --release
```

- **dongle**：用调试器 `cargo run --release --bin central`，或拖 `.uf2` 进 bootloader 盘。
- **左/右半体**：分别烧 `peripheral` / `peripheral2`。
- RMK 在插 USB 时自动切到 USB 模式；烧录完记得拔掉 USB 线再上电，
  让 dongle 与两个半体通过 BLE 自动配对。

## 工作原理小结

RMK 的 `#[rmk_central]` / `#[rmk_peripheral(id=N)]` 宏会读取
`keyboard.toml` 自动生成键盘 + BLE + USB + 显示逻辑。显示部分由
`DisplayProcessor` 订阅键盘事件，每当有变化（或到达 `render_interval`）就调用
`BongoCatRenderer::render`，把当前 `RenderContext` 画到 OLED 上——所以我们只
要实现一个 `render` 方法就能让小屏显示会随打字敲鼓的 bongocat。
