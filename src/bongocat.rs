//! Toykit v2 dongle OLED renderer — adapter layer for `rmk-dongle-display`.
//!
//! `rmk-dongle-display` is a standalone, allocation-free `no_std` rendering
//! library that owns a state machine, a default dongle scene (output / link /
//! profile / layer / WPM / battery / modifiers / indicators / sleep + a
//! bongocat animation) and a page-major 1-bpp framebuffer with damage
//! tracking.
//!
//! Toykit-v2's `#[rmk_central]` macro auto-generates a `DisplayProcessor`
//! that drives our `DisplayRenderer<BinaryColor>` hook on every render
//! tick. This file bridges the two by:
//!
//!   1. Mapping RMK's `RenderContext` fields into `DisplayEvent` values
//!      that the source crate's `DongleDisplay` can consume. With the
//!      `rmk-types` direct dep we match the `BatteryStatus` /
//!      `ChargeState` / `BleState` enums and forward:
//!        - controller battery (with `charge_state`)
//!        - per-peripheral batteries (split dongle topology)
//!        - BLE transport (advertising / connected / inactive) -> profile +
//!          link state
//!   2. Letting the source crate perform state reduction + scene rendering
//!      (including the bongocat animation cadence and dirty-page tracking).
//!   3. Blitting the source crate's `MonoFramebuffer` straight onto RMK's
//!      `DrawTarget` by wrapping its page-major LSB-top bytes in an
//!      `embedded_graphics::ImageRaw` (which defaults to the same byte
//!      order, so no per-pixel conversion is needed).
//!
//! The `BongoCatRenderer` name is kept verbatim so the existing
//! `renderer = "toykit_dongle::BongoCatRenderer"` line in `keyboard.toml`
//! keeps resolving; no macro or toml changes are required.
//!
//! Custom-renderer dependencies in `Cargo.toml`:
//!   * `rmk-types`           — direct dep for matching `BatteryStatus` /
//!     `BleState` enums (same rev as `rmk`).
//!   * `dongle-display`      — the source crate, pinned to a single rev
//!     and only the `street-fighter` feature (so its own `rmk` is *not*
//!     pulled in and there is no conflict with Toykit-v2's pinned
//!     `rmk` rev).
//!   * `embedded-graphics 0.8` — for `ImageRaw` / `Image`.
//!
//! Switch animation back to the bongo-cat sprite by changing the feature
//! in `Cargo.toml`:
//!   `features = ["bongo-cat"]`  (mutually exclusive with `street-fighter`).

#![allow(dead_code)]

// `rmk-dongle-display` re-exports its public surface from two places:
//   - crate root: `DongleDisplay`, `RenderResult`, `MonoFramebuffer`,
//     `FramebufferError`, `ModifierStyle`, animation handles.
//   - `display` submodule: every semantic type (`BatterySource`,
//     `DisplayEvent`, `LinkState`, `OutputKind`, `Size`, `DisplayState`,
//     `StateDiff`, etc.) is reached via `dongle_display::display::*`.
use dongle_display::display::{
    BatterySource, DisplayEvent, LinkState, OutputKind, Size as DSize,
};
use dongle_display::{DongleDisplay, ModifierStyle};
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::*,
};
use rmk::display::{DisplayRenderer, RenderContext};
use rmk_types::battery::{BatteryStatus, ChargeState};
use rmk_types::ble::BleState;

/// OLED resolution declared in `keyboard.toml` -> `[split.central.display]`.
const SCREEN_W: u16 = 128;
const SCREEN_H: u16 = 64;

/// Per-tick increment (ms) used as a monotonic "now" for the source crate's
/// scene scheduler. The macro-generated `DisplayProcessor` invokes
/// `render()` roughly every `render_interval` (40 ms by default), so a
/// fixed step is good enough to drive the bongocat animation cadence.
const TICK_STEP_MS: u64 = 40;

/// Wrapper that holds the source crate's `DongleDisplay` plus a simple
/// tick counter. The display is built lazily on the first render because
/// `Default::default()` cannot fail.
///
/// `RMK constructs the renderer with `BongoCatRenderer::default()` per the
/// `display.renderer` field in `keyboard.toml`.
pub struct BongoCatRenderer {
    inner: Option<DongleDisplay>,
    tick_ms: u64,
}

impl Default for BongoCatRenderer {
    fn default() -> Self {
        Self {
            inner: None,
            tick_ms: 0,
        }
    }
}

impl BongoCatRenderer {
    fn ensure(&mut self) -> Option<&mut DongleDisplay> {
        if self.inner.is_none() {
            self.inner = DongleDisplay::new_with_modifier_style(
                DSize::new(SCREEN_W, SCREEN_H),
                ModifierStyle::Pc,
            )
            .ok();
        }
        self.inner.as_mut()
    }
}

/// Toykit-v2's `ctx.caps_lock` / `ctx.num_lock` -> u8 bitfield.
/// The source crate expects `bit0 = num_lock`, `bit1 = caps_lock`.
fn indicators_from_ctx(ctx: &RenderContext) -> u8 {
    u8::from(ctx.num_lock) | (u8::from(ctx.caps_lock) << 1)
}

/// `ctx.modifiers: ModifierCombination` -> u8 bitfield. The bit layout matches
/// what the source crate's `ModifiersChanged` event expects (LCtrl/LShift/
/// LAlt/LGui/RCtrl/RShift/RAlt/RGui), so we just forward RMK's own
/// `into_bits()`. The `ModifierCombination` type is reached through RMK's
/// re-export of `rmk_types`; the `BatteryStatus`/`BleState` enums below are
/// reached through the direct `rmk-types` dep added in `Cargo.toml`.
fn modifiers_to_bitfield(m: rmk::types::modifier::ModifierCombination) -> u8 {
    m.into_bits()
}

/// Map the dongle's own battery (a `BatteryStatus` enum wrapped in a
/// `BatteryStatusEvent` tuple struct) to a single `DisplayEvent`.
fn controller_battery_event(ctx: &RenderContext) -> DisplayEvent {
    match ctx.battery.0 {
        BatteryStatus::Unavailable => DisplayEvent::BatteryUnavailable {
            source: BatterySource::Controller,
        },
        BatteryStatus::Available {
            charge_state,
            level,
        } => DisplayEvent::BatteryChanged {
            source: BatterySource::Controller,
            percent: level.unwrap_or(0).min(100),
            charging: matches!(charge_state, ChargeState::Charging),
        },
    }
}

/// Translate RMK's `BleStatus { profile, state }` into the source crate's
/// (OutputKind, LinkState). When BLE is inactive (sleep / USB-only) we
/// fall back to USB so the dongle's top-bar still shows a transport pill.
///
/// Gated on the `rmk` crate's `_ble` feature — `nrf52840_ble` already
/// enables it transitively for this firmware.
#[cfg(feature = "_ble")]
fn transport_from_ctx(ctx: &RenderContext) -> (OutputKind, LinkState) {
    let ble = ctx.ble_status;
    match ble.state {
        BleState::Advertising => (OutputKind::Ble { profile: ble.profile }, LinkState::Searching),
        BleState::Connected => (OutputKind::Ble { profile: ble.profile }, LinkState::Connected),
        BleState::Inactive => (OutputKind::Usb, LinkState::Disconnected),
    }
}

/// Without the `_ble` feature, RMK doesn't surface `ble_status`. Fall back
/// to "USB connected" so the dongle still draws a sensible transport pill.
#[cfg(not(feature = "_ble"))]
fn transport_from_ctx(_ctx: &RenderContext) -> (OutputKind, LinkState) {
    (OutputKind::Usb, LinkState::Connected)
}

/// Refine the dongle's overall link state by looking at which peripherals
/// are currently attached. "All connected" -> Connected, "some connected"
/// -> Searching (partial), "Unconnected" -> Disconnected.
#[cfg(feature = "split")]
fn peripheral_link_state(ctx: &RenderContext) -> LinkState {
    let mut any = false;
    let mut all = true;
    for &c in &ctx.peripherals_connected {
        any |= c;
        all &= c;
    }
    if all {
        LinkState::Connected
    } else if any {
        LinkState::Searching
    } else {
        LinkState::Disconnected
    }
}

/// Without `split`, peripherals_connected isn't in the context; trust the
/// transport-only signal from `transport_from_ctx`.
#[cfg(not(feature = "split"))]
fn peripheral_link_state(_ctx: &RenderContext) -> LinkState {
    LinkState::Connected
}

/// Forward each peripheral's battery to the source crate as a
/// `BatterySource::Peripheral(i)` event. Only compiled when both
/// `split` and `_ble` are on — otherwise the `peripheral_batteries`
/// field isn't in `RenderContext`.
#[cfg(all(feature = "split", feature = "_ble"))]
fn push_peripheral_batteries(display: &mut DongleDisplay, ctx: &RenderContext) {
    for (i, evt) in ctx.peripheral_batteries.iter().enumerate() {
        let source = BatterySource::Peripheral(i as u8);
        let ev = match evt.0 {
            BatteryStatus::Unavailable => DisplayEvent::BatteryUnavailable { source },
            BatteryStatus::Available {
                charge_state,
                level,
            } => DisplayEvent::BatteryChanged {
                source,
                percent: level.unwrap_or(0).min(100),
                charging: matches!(charge_state, ChargeState::Charging),
            },
        };
        display.apply(ev);
    }
}

impl DisplayRenderer<BinaryColor> for BongoCatRenderer {
    fn render<D: DrawTarget<Color = BinaryColor>>(
        &mut self,
        ctx: &RenderContext,
        target: &mut D,
    ) {
        // ---- 0. Snapshot & advance the local tick BEFORE borrowing `inner` ----
        //
        // `ensure()` returns `Option<&mut DongleDisplay>` which is derived
        // from `&mut self`; holding that borrow across any further access to
        // `self.tick_ms` would trip E0503/E0506. Grabbing the tick first and
        // pre-incrementing keeps the borrow of `inner` scoped to the rest of
        // the function body. If `ensure()` fails we have simply advanced the
        // animation clock by one frame — harmless, the next successful render
        // continues from the new tick.
        let now_ms = self.tick_ms;
        self.tick_ms = self.tick_ms.wrapping_add(TICK_STEP_MS);

        let Some(display) = self.ensure() else {
            return;
        };

        // ---- 1. RenderContext -> DisplayEvent stream ----
        // The BLE side tells us the host transport + profile; the peripheral
        // side tells us whether the two halves are linked. The peripheral
        // state is authoritative for the "linked to the halves" indicator.
        let (output, _link_from_ble) = transport_from_ctx(ctx);
        let link = peripheral_link_state(ctx);

        display.apply(DisplayEvent::OutputChanged { output });
        display.apply(DisplayEvent::ConnectionChanged { state: link });
        // Update profile whenever the BLE output changed to a BLE/ESB kind.
        if let OutputKind::Ble { profile } | OutputKind::Esb { profile } = output {
            display.apply(DisplayEvent::ProfileChanged { profile });
        }
        display.apply(DisplayEvent::LayerChanged { layer: ctx.layer });
        display.apply(DisplayEvent::WpmChanged { wpm: ctx.wpm });
        display.apply(DisplayEvent::ModifiersChanged {
            modifiers: modifiers_to_bitfield(ctx.modifiers),
        });
        display.apply(DisplayEvent::IndicatorsChanged {
            indicators: indicators_from_ctx(ctx),
        });
        display.apply(DisplayEvent::SleepChanged {
            sleeping: ctx.sleeping,
        });
        display.apply(controller_battery_event(ctx));

        // Push each split peripheral's battery (no-op when `split`/`_ble` are off).
        #[cfg(all(feature = "split", feature = "_ble"))]
        push_peripheral_batteries(display, ctx);

        // `key_press_latch` is cleared by RMK after every render, so each
        // edge triggers exactly one frame flip on the bongocat animation.
        if ctx.key_press_latch {
            display.apply(DisplayEvent::AnimationTrigger { id: 0 });
        }

        // ---- 2. State reduction + scene render inside the source crate ----
        // `now_ms` was captured before `ensure()` so `display` can be used
        // freely here without re-borrowing `self`.
        if display.render(now_ms).is_err() {
            return;
        }

        // ---- 3. Blit the page-major LSB-top MonoFramebuffer onto RMK's DrawTarget ----
        //
        // `MonoFramebuffer::bytes()` exposes the raw SSD1306 page-major
        // layout: each page (`height/8` rows) is stored as `width` bytes,
        // and within each byte bit `0` is the topmost row. That is exactly
        // the byte order that `ImageRaw::new(data, width)` defaults to for
        // `BinaryColor`, so we can hand the slice straight in and skip the
        // 8 192-pixel `draw_iter` path.
        let fb = display.framebuffer();
        let raw: ImageRaw<'_, BinaryColor> =
            ImageRaw::new(fb.bytes(), u32::from(fb.size().width));
        let image: Image<'_, ImageRaw<'_, BinaryColor>> = Image::new(&raw, Point::zero());
        let _ = image.draw(target);
    }
}
