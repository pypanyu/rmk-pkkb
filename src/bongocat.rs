//! Bongocat OLED renderer for the Toykit v2 dongle.
//!
//! Implements [`rmk::display::DisplayRenderer`] for a monochrome (BinaryColor)
//! display. The cat "drums" whenever a key is pressed:
//!   * a fresh key press (`key_press_latch`) flips the animation frame,
//!   * a held key (`key_pressed`) keeps the paws down on the bongos,
//!   * when idle, the cat gently bobs using a slow tick counter.
//!
//! Designed for a 128x64 SSD1306.
use core::fmt::Write as _;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle, Triangle},
    text::Text,
};
use heapless::String;
use rmk::display::{DisplayRenderer, RenderContext};
/// Animated bongocat. Holds a tiny bit of state between renders.
#[derive(Default)]
pub struct BongoCatRenderer {
    /// Toggles between the two drum frames on each fresh key press.
    frame: bool,
    /// Edge tracking for `key_press_latch`.
    last_latch: bool,
    /// Free-running counter used to idle-animate (bob) when no key is active.
    idle_tick: u32,
}

impl DisplayRenderer<BinaryColor> for BongoCatRenderer {
    fn render<D: DrawTarget<Color = BinaryColor>>(&mut self, ctx: &RenderContext, display: &mut D) {
        display.clear(BinaryColor::Off).ok();
        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

        // --- 1. 右上角：电量显示 Battery %
        let mut bat_str: String<16> = String::new();
        write!(&mut bat_str, "{}%", ctx.battery).ok();
        Text::new(&bat_str, Point::new(96, 10), style).draw(display).ok();

        // --- 2. 左侧：图层名称映射，替代原来 L数字
        let layer_name = match ctx.layer {
            0 => "NLCK",
            1 => "LOWER",
            2 => "RAISE",
            3 => "ADJUST",
            _ => "UNK",
        };
        Text::new(layer_name, Point::new(4, 42), style).draw(display).ok();

        // --- Animation state machine (原邦戈猫动画完全保留，不改动) -------------------------------------
        if ctx.key_press_latch && !self.last_latch {
            // A new key was pressed since the last render -> flip the frame,
            // which makes the cat look like it is drumming.
            self.frame = !self.frame;
        }
        self.last_latch = ctx.key_press_latch;
        self.idle_tick = self.idle_tick.wrapping_add(1);
        let down = if ctx.key_pressed {
            true // key held -> paws stay on the bongos
        } else if ctx.key_press_latch {
            self.frame // just pressed -> show the toggled frame
        } else {
            // Idle: bob slowly (toggle every 24 renders; render_interval=40ms
            // => ~1s per bob).
            (self.idle_tick / 24) % 2 == 0
        };
        // 绘制邦戈猫，坐标微调，放到屏幕右下区域，和参考图对齐
        draw_cat(display, down);
    }
}

/// Draw a simple bongocat. `down == true` puts the paws on the bongos.
fn draw_cat<D: DrawTarget<Color = BinaryColor>>(display: &mut D, down: bool) {
    let fill = PrimitiveStyle::with_fill(BinaryColor::On);
    let hole = PrimitiveStyle::with_fill(BinaryColor::Off);
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
    // Head
    Circle::new(Point::new(74, 22), 20).into_styled(fill).draw(display).ok();
    // Ears
    Triangle::new(Point::new(74, 22), Point::new(69, 15), Point::new(81, 21))
        .into_styled(fill)
        .draw(display)
        .ok();
    Triangle::new(Point::new(94, 22), Point::new(99, 15), Point::new(87, 21))
        .into_styled(fill)
        .draw(display)
        .ok();
    // Eyes (punch holes so they read as "off" pixels)
    Circle::new(Point::new(81, 28), 2).into_styled(hole).draw(display).ok();
    Circle::new(Point::new(89, 28), 2).into_styled(hole).draw(display).ok();
    // Body
    Rectangle::new(Point::new(70, 40), Size::new(28, 16))
        .into_styled(fill)
        .draw(display)
        .ok();
    // Bongos
    Circle::new(Point::new(56, 56), 9).into_styled(fill).draw(display).ok();
    Circle::new(Point::new(106, 56), 9).into_styled(fill).draw(display).ok();
    // Arms + paws. Paws are raised when `down == false`, on the bongos when true.
    let paw_y = if down { 52 } else { 38 };
    // Left arm
    Line::new(Point::new(74, 44), Point::new(62, paw_y))
        .into_styled(stroke)
        .draw(display)
        .ok();
    Circle::new(Point::new(60, paw_y.saturating_sub(2)), 4)
        .into_styled(fill)
        .draw(display)
        .ok();
    // Right arm
    Line::new(Point::new(94, 44), Point::new(106, paw_y))
        .into_styled(stroke)
        .draw(display)
        .ok();
    Circle::new(Point::new(104, paw_y.saturating_sub(2)), 4)
        .into_styled(fill)
        .draw(display)
        .ok();
}
