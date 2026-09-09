#!/usr/bin/env python3
"""
Draw a CC0-style sitting-cat-plays-bongo sprite (96x48) adapted from
Shepardskin's CC0 Cat Sprites (OpenGameArt, 2014):
    https://opengameart.org/node/21390
(CC0 1.0 Universal — free to adapt, no attribution required.)

Layout (96 cols x 48 rows, top-left origin, y grows downward):
  - Vertical split (when placed into 128x64):
      Row  0..15 : top text strip (WPM/battery/Layer drawn by Rust)
      Row 16..63 : this bitmap (placed at y=16, x=(128-96)/2=16)
  - Scene (96x48) contains:
      - left bongo drum (cx~22, cy~36)
      - right bongo drum (cx~76, cy~36)
      - sitting cat in the middle facing RIGHT:
          body oval at  (48, 24), head at (60, 14)
          2 ears on top, 1 eye, curl-tail on the left, 2 legs, 2 arms

We emit two animation frames so the toykit can flip on each key press:
  - CAT_DOWN : both paws down on the bongos
  - CAT_UP   : left paw raised (mid-strike)
"""
from PIL import Image, ImageDraw

W, H = 80, 40

def new_canvas():
    return Image.new("L", (W, H), 0)

def to_binary(img_l):
    return img_l.point(lambda v: 1 if v >= 128 else 0, mode="1")

def ellipse_filled(img, cx, cy, rx, ry, fill=255):
    ImageDraw.Draw(img).ellipse(
        [cx - rx, cy - ry, cx + rx, cy + ry], fill=fill)

def ellipse_outline(img, cx, cy, rx, ry, fill=255):
    ImageDraw.Draw(img).ellipse(
        [cx - rx, cy - ry, cx + rx, cy + ry], outline=fill)

def triangle(img, pts, fill=255):
    ImageDraw.Draw(img).polygon(pts, fill=fill)


def draw_cat(img, *, left_paw_up=False):
    """Draw CC0-style cat sitting between two bongos, facing right."""
    d = ImageDraw.Draw(img)

    # ===== Floor baseline (subtle horizontal line — helps the eye) =====
    d.line([(2, 36), (W - 2, 36)], fill=255, width=1)

    # ===== Left bongo =====
    L_cx, L_cy = 18, 30
    L_rx, L_ry = 9, 7
    ellipse_filled(img, L_cx, L_cy, L_rx, L_ry)               # drum body
    ellipse_filled(img, L_cx, L_cy - 2, L_rx - 1, 2, 0)       # drum hole / opening (black)
    d.line([(L_cx - L_rx + 1, L_cy + 1),
            (L_cx + L_rx - 1, L_cy + 1)], fill=255, width=1) # band

    # ===== Right bongo (bigger) =====
    R_cx, R_cy = 64, 30
    R_rx, R_ry = 11, 8
    ellipse_filled(img, R_cx, R_cy, R_rx, R_ry)
    ellipse_filled(img, R_cx, R_cy - 2, R_rx - 1, 2, 0)
    d.line([(R_cx - R_rx + 1, R_cy + 1),
            (R_cx + R_rx - 1, R_cy + 1)], fill=255, width=1)

    # ===== Cat body (oval, slightly squashed) =====
    body_cx, body_cy = 40, 22
    body_rx, body_ry = 11, 8
    ellipse_filled(img, body_cx, body_cy, body_rx, body_ry)
    # white belly patch (cartoon contrast)
    ellipse_filled(img, body_cx + 1, body_cy + 2,
                   body_rx - 4, body_ry - 4, 255)

    # ===== Head (positioned above + to the right) =====
    head_cx, head_cy = body_cx + 9, body_cy - 7
    head_r = 6
    ellipse_filled(img, head_cx, head_cy, head_r, head_r)
    # white muzzle (right side of face)
    ellipse_filled(img, head_cx + 3, head_cy + 1, 3, 3, 255)
    # single eye (on the right side of the head since cat faces right)
    ellipse_filled(img, head_cx + 2, head_cy - 1, 1, 1, 0)
    # nose - tiny black dot
    ellipse_filled(img, head_cx + 5, head_cy + 2, 1, 1, 0)
    # white whisker hint (single short stroke, right)
    d.line([(head_cx + 5, head_cy + 3),
            (head_cx + 7, head_cy + 4)], fill=0, width=1)

    # ===== Ears (two triangles on top of head) =====
    triangle(img, [(head_cx - 4, head_cy - 4),
                   (head_cx - 2, head_cy - 8),
                   (head_cx - 0, head_cy - 4)])     # rear (left) ear
    triangle(img, [(head_cx + 1, head_cy - 4),
                   (head_cx + 4, head_cy - 9),
                   (head_cx + 6, head_cy - 4)])     # front (right) ear

    # ===== Tail (curls up over body toward left) =====
    d.line([(body_cx - body_rx, body_cy + 0),
            (body_cx - body_rx - 3, body_cy - 4),
            (body_cx - body_rx - 2, body_cy - 8),
            (body_cx - body_rx + 2, body_cy - 7)], fill=255, width=2)

    # ===== Back ridge (CC0 sprite has these little back-spikes) =====
    for dx in (-6, -3, 0, 3):
        d.line([(body_cx + dx, body_cy - body_ry),
                (body_cx + dx, body_cy - body_ry - 2)], fill=255, width=1)

    # ===== Legs (two visible legs, ending on respective drums) =====
    # back leg → left drum
    d.line([(body_cx - 7, body_cy + body_ry - 1),
            (L_cx + 3,  L_cy - 1)], fill=255, width=2)
    # front leg → right drum
    d.line([(body_cx + 7, body_cy + body_ry - 1),
            (R_cx - 4,  R_cy - 1)], fill=255, width=2)

    # ===== Arms =====
    # right arm always reaching to right drum
    d.line([(body_cx + 9, body_cy + 4),
            (R_cx - 5, R_cy - R_ry + 1)], fill=255, width=2)
    ellipse_filled(img, R_cx - 5, R_cy - R_ry + 1, 2, 1, 255)

    # left arm:  either down on left drum or raised up (mid-strike)
    if left_paw_up:
        # raised up to the left, arm bent upward
        d.line([(body_cx + 6, body_cy + 3),
                (body_cx + 1, body_cy - 7)], fill=255, width=2)
        d.line([(body_cx + 1, body_cy - 7),
                (L_cx + 6, body_cy - 9)], fill=255, width=2)
        ellipse_filled(img, L_cx + 6, body_cy - 9, 2, 2, 255)
    else:
        # down on left drum
        d.line([(body_cx + 6, body_cy + 5),
                (L_cx + 5, L_cy - L_ry + 1)], fill=255, width=2)
        ellipse_filled(img, L_cx + 5, L_cy - L_ry + 1, 2, 2, 255)


def emit_frame(name, left_paw_up):
    img = new_canvas()
    draw_cat(img, left_paw_up=left_paw_up)
    img.save(f"cat_source/{name}_art.png")
    bw = to_binary(img)
    bw.save(f"cat_source/{name}_bw.png")
    pixels = bw.load()
    byte_rows = []
    # 96 cols / 8 = 12 bytes per row; 48 rows => 12*48 = 576 bytes/frame
    for y in range(H):
        for col in range(0, W, 8):
            b = 0
            for bit in range(8):
                if col + bit < W and pixels[col + bit, y]:
                    b |= 1 << (7 - bit)
            byte_rows.append(b)
    return byte_rows


def fmt_const(name, data):
    lines = []
    for i in range(0, len(data), 12):
        chunk = ", ".join(f"0x{b:02X}" for b in data[i:i + 12])
        lines.append("    " + chunk + ",")
    body = "\n".join(lines)
    return f"const {name}: [u8; {len(data)}] = [\n{body}\n];\n"


if __name__ == "__main__":
    out_down = emit_frame("frame_down", left_paw_up=False)
    out_up   = emit_frame("frame_up",   left_paw_up=True)

    text = (
        "// Auto-generated by gen_cc0_cat.py — 80x40, 1 bpp, MSB-first.\n"
        "// Adapted from Shepardskin's CC0 Cat Sprites (OpenGameArt, 2014):\n"
        "//   https://opengameart.org/node/21390  (CC0 1.0 Universal).\n\n"
        f"pub const CAT_W: u32 = {W};\n"
        f"pub const CAT_H: u32 = {H};\n"
        f"pub const CAT_BYTES: usize = {W // 8} * {H};\n\n"
        + fmt_const("CAT_DOWN", out_down) + "\n"
        + fmt_const("CAT_UP",   out_up)
    )
    with open("cat_source/rust_const_arrays.txt", "w", encoding="utf-8") as f:
        f.write(text)
    # print first 30 lines only
    for line in text.splitlines()[:30]:
        print(line)
    print("... (truncated; full content saved to cat_source/rust_const_arrays.txt)")
