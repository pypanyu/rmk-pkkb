//! Bongocat OLED renderer for the Toykit v2 dongle.
//!
//! Implements [`rmk::display::DisplayRenderer`] for a monochrome (BinaryColor)
//! display. The cat "drums" whenever a key is pressed:
//!   * a fresh key press (`key_press_latch`) flips the animation frame,
//!   * a held key (`key_pressed`) keeps the paws down on the bongos,
//!   * when idle, the cat gently bobs using a slow tick counter.
//!
//! Designed for a 128x64 SSD1306. If you use a 128x32 panel, shrink the
//! coordinates / drop the status line (see `render`).

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

        // --- Animation state machine -------------------------------------
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

        draw_cat(display, down);

        // --- Status line (layer + WPM) -----------------------------------
        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        let mut line: String<32> = String::new();
        write!(&mut line, "WPM {}  L{}", ctx.wpm, ctx.layer).ok();
        Text::new(&line, Point::new(2, 62), style).draw(display).ok();
    }
}

/// Draw a simple bongocat. `down == true` puts the paws on the bongos.
fn draw_cat<D: DrawTarget<Color = BinaryColor>>(display: &mut D, down: bool) {
    let fill = PrimitiveStyle::with_fill(BinaryColor::On);
    let hole = PrimitiveStyle::with_fill(BinaryColor::Off);
    let stroke = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

    // Head
    Circle::new(Point::new(54, 10), 20).into_styled(fill).draw(display).ok();
    // Ears
    Triangle::new(Point::new(54, 10), Point::new(49, 3), Point::new(61, 9))
        .into_styled(fill)
        .draw(display)
        .ok();
    Triangle::new(Point::new(74, 10), Point::new(79, 3), Point::new(67, 9))
        .into_styled(fill)
        .draw(display)
        .ok();
    // Eyes (punch holes so they read as "off" pixels)
    Circle::new(Point::new(61, 16), 2).into_styled(hole).draw(display).ok();
    Circle::new(Point::new(69, 16), 2).into_styled(hole).draw(display).ok();

    // Body
    Rectangle::new(Point::new(50, 28), Size::new(28, 16))
        .into_styled(fill)
        .draw(display)
        .ok();

    // Bongos
    Circle::new(Point::new(36, 44), 9).into_styled(fill).draw(display).ok();
    Circle::new(Point::new(86, 44), 9).into_styled(fill).draw(display).ok();

    // Arms + paws. Paws are raised when `down == false`, on the bongos when true.
    let paw_y = if down { 40 } else { 26 };
    // Left arm
    Line::new(Point::new(54, 32), Point::new(42, paw_y))
        .into_styled(stroke)
        .draw(display)
        .ok();
    Circle::new(Point::new(40, paw_y.saturating_sub(2)), 4)
        .into_styled(fill)
        .draw(display)
        .ok();
    // Right arm
    Line::new(Point::new(74, 32), Point::new(86, paw_y))
        .into_styled(stroke)
        .draw(display)
        .ok();
    Circle::new(Point::new(84, paw_y.saturating_sub(2)), 4)
        .into_styled(fill)
        .draw(display)
        .ok();
}
