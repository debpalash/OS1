#!/usr/bin/env python3
"""Generate the OS 1 Plymouth theme PNGs (wordmark, glow ring, progress, dot).

Pre-rendering everything means the initramfs needs no fonts or label plugin —
the splash works with the bare plymouth script module.

Run from this directory:  python3 gen_assets.py
"""
import math

import cairo

CORAL = (0xD1 / 255, 0x68 / 255, 0x4E / 255)
WARM_WHITE = (0xF0 / 255, 0xE0 / 255, 0xD6 / 255)


def surface(w, h):
    return cairo.ImageSurface(cairo.FORMAT_ARGB32, w, h)


def wordmark():
    """'OS 1' in a light humanist sans with wide letter-spacing."""
    w, h = 1200, 360
    s = surface(w, h)
    c = cairo.Context(s)
    c.select_font_face("Noto Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_NORMAL)
    c.set_font_size(220)
    text, spacing = "OS 1", 28
    # measure with manual letter-spacing so we can center
    widths = []
    for ch in text:
        ext = c.text_extents(ch)
        widths.append(ext.x_advance)
    total = sum(widths) + spacing * (len(text) - 1)
    x = (w - total) / 2
    base = h / 2 + 220 * 0.36  # optical vertical center
    c.set_source_rgba(*WARM_WHITE, 1.0)
    for ch, adv in zip(text, widths):
        c.move_to(x, base)
        c.show_text(ch)
        x += adv + spacing
    s.write_to_png("wordmark.png")


def ring():
    """Soft breathing glow ring — concentric strokes with alpha falloff."""
    size, cx, r0 = 560, 280, 168
    s = surface(size, size)
    c = cairo.Context(s)
    # outer glow: many faint wide strokes
    for i in range(40):
        t = i / 39
        c.set_line_width(3 + 26 * (1 - t))
        c.set_source_rgba(*CORAL, 0.012 + 0.05 * t)
        c.arc(cx, cx, r0 + 14 * (1 - t), 0, 2 * math.pi)
        c.stroke()
    # crisp core ring
    c.set_line_width(3.5)
    c.set_source_rgba(*CORAL, 0.95)
    c.arc(cx, cx, r0, 0, 2 * math.pi)
    c.stroke()
    s.write_to_png("ring.png")


def capsule(name, color, alpha):
    """Rounded 64x8 capsule, stretched horizontally by the script."""
    w, h, r = 64, 8, 4
    s = surface(w, h)
    c = cairo.Context(s)
    c.set_source_rgba(*color, alpha)
    c.arc(r, h / 2, r, math.pi / 2, 3 * math.pi / 2)
    c.arc(w - r, h / 2, r, -math.pi / 2, math.pi / 2)
    c.close_path()
    c.fill()
    s.write_to_png(name)


def dot():
    """Soft dot for password bullets."""
    size = 18
    s = surface(size, size)
    c = cairo.Context(s)
    g = cairo.RadialGradient(size / 2, size / 2, 1, size / 2, size / 2, size / 2)
    g.add_color_stop_rgba(0, *WARM_WHITE, 1)
    g.add_color_stop_rgba(0.7, *WARM_WHITE, 0.9)
    g.add_color_stop_rgba(1, *WARM_WHITE, 0)
    c.set_source(g)
    c.arc(size / 2, size / 2, size / 2, 0, 2 * math.pi)
    c.fill()
    s.write_to_png("dot.png")


wordmark()
ring()
capsule("progress_track.png", WARM_WHITE, 0.14)
capsule("progress_fill.png", CORAL, 0.95)
dot()
print("assets written")
