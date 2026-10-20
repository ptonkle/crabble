#!/usr/bin/env python3
"""Writes data/glyphs.json: A-Z and 0-9 as convex polygons, a chunky
rounded sans for the tiles (crates/core/src/glyph.rs reads the file).

data/glyphs.json is the source of truth the game reads. This script is how
its first version was drawn: each glyph is built from a few shapes (bars,
thick lines, arcs of an elliptical ring), every piece is clipped to the
glyph's box and rounded to the grid, and each is checked convex. Edit a
glyph here and run it again, or edit the JSON by hand.

    python3 tools/glyphs/make_glyphs.py          writes data/glyphs.json
    python3 tools/glyphs/sheet.py OUT.png        draws every glyph, to look at

Grid: the cap height is 100 units, x to the right, y up from the baseline.
Vertical strokes are V units thick and horizontal ones H, as a rounded
sans's are (its horizontals a little thinner).
"""
import json
import math
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
HEIGHT = 100
GAP = 10
LIFT_UM = 400
V = 22  # vertical stroke
H = 18  # horizontal stroke


def rect(x0, y0, x1, y1):
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def line(p, q, w):
    """A bar of width w from p to q, square-ended."""
    dx, dy = q[0] - p[0], q[1] - p[1]
    n = math.hypot(dx, dy)
    ox, oy = -dy / n * w / 2, dx / n * w / 2
    return [(p[0] + ox, p[1] + oy), (p[0] - ox, p[1] - oy), (q[0] - ox, q[1] - oy), (q[0] + ox, q[1] + oy)]


def ring(cx, cy, rx, ry, a0, a1, n, tx=V, ty=H):
    """Pieces of an elliptical ring from angle a0 to a1 (degrees, either
    way round), n of them; the ring is tx thick across and ty up."""
    out = []
    for k in range(n):
        t0 = math.radians(a0 + (a1 - a0) * k / n)
        t1 = math.radians(a0 + (a1 - a0) * (k + 1) / n)
        o = lambda t, sx, sy: (cx + sx * math.cos(t), cy + sy * math.sin(t))
        out.append([o(t0, rx, ry), o(t1, rx, ry), o(t1, rx - tx, ry - ty), o(t0, rx - tx, ry - ty)])
    return out


def clip(poly, w):
    """Sutherland-Hodgman against the box 0..w by 0..HEIGHT."""
    def cut(points, inside, cross):
        out = []
        for i, p in enumerate(points):
            q = points[(i + 1) % len(points)]
            if inside(p):
                out.append(p)
                if not inside(q):
                    out.append(cross(p, q))
            elif inside(q):
                out.append(cross(p, q))
        return out

    def at_x(x):
        return lambda p, q: (x, p[1] + (q[1] - p[1]) * (x - p[0]) / (q[0] - p[0]))

    def at_y(y):
        return lambda p, q: (p[0] + (q[0] - p[0]) * (y - p[1]) / (q[1] - p[1]), y)

    poly = cut(poly, lambda p: p[0] >= 0, at_x(0))
    poly = cut(poly, lambda p: p[0] <= w, at_x(w))
    poly = cut(poly, lambda p: p[1] >= 0, at_y(0))
    poly = cut(poly, lambda p: p[1] <= HEIGHT, at_y(HEIGHT))
    return poly


def side(a, b, c):
    return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])


def convex(p):
    """As the core checks it (crates/core/src/glyph.rs)."""
    if len(p) < 3:
        return False
    turned = False
    for i in range(len(p)):
        a, b = p[i], p[(i + 1) % len(p)]
        if a == b:
            return False
        s = [side(a, b, c) for c in p]
        if any(x > 0 for x in s) and any(x < 0 for x in s):
            return False
        turned |= any(x != 0 for x in s)
    return turned


def tidy(poly, w):
    """Clipped, on the grid, without repeated corners or straight-run
    middles."""
    pts = [(round(x), round(y)) for x, y in clip(poly, w)]
    out = []
    for p in pts:
        if not out or out[-1] != p:
            out.append(p)
    while len(out) > 1 and out[0] == out[-1]:
        out.pop()
    changed = True
    while changed and len(out) > 3:
        changed = False
        for i in range(len(out)):
            a, b, c = out[i - 1], out[i], out[(i + 1) % len(out)]
            if side(a, b, c) == 0:
                out.pop(i)
                changed = True
                break
    return out


def flip(polys, w):
    """Turned half round (6 into 9)."""
    return [[(w - x, HEIGHT - y) for x, y in p] for p in polys]


def bowl(x_stem, top, bottom, right, n=10):
    """A D-shaped bowl off a stem: bars from the stem along its top and
    bottom, and the right half of a ring between them."""
    cy = (top + bottom) / 2
    ry = (top - bottom) / 2
    cx = x_stem + (right - x_stem) * 0.36
    rx = right - cx
    return [rect(0, top - H, cx + 1, top), rect(0, bottom, cx + 1, bottom + H)] + ring(cx, cy, rx, ry, 90, -90, n)


def glyphs():
    g = {}
    # letters
    g["A"] = (90, [[(0, 0), (V, 0), (58, 100), (32, 100)], [(68, 0), (90, 0), (58, 100), (32, 100)], rect(12, 16, 78, 16 + H)])
    g["B"] = (76, [rect(0, 0, V, 100)] + bowl(V, 100, 41, 70) + bowl(V, 59, 0, 76))
    g["C"] = (82, ring(44, 50, 44, 50, 42, 318, 18))
    g["D"] = (82, [rect(0, 0, V, 100), rect(0, 0, 34, H), rect(0, 100 - H, 34, 100)] + ring(32, 50, 50, 50, 90, -90, 12))
    g["E"] = (64, [rect(0, 0, V, 100), rect(0, 100 - H, 64, 100), rect(0, 41, 56, 59), rect(0, 0, 64, H)])
    g["F"] = (62, [rect(0, 0, V, 100), rect(0, 100 - H, 62, 100), rect(0, 38, 54, 38 + H)])
    g["G"] = (86, ring(44, 50, 44, 50, 45, 360, 18) + [rect(46, 40, 86, 40 + H), rect(64, 6, 86, 50)])
    g["H"] = (80, [rect(0, 0, V, 100), rect(58, 0, 80, 100), rect(0, 41, 80, 59)])
    g["I"] = (V, [rect(0, 0, V, 100)])
    g["J"] = (64, [rect(42, 30, 64, 100)] + ring(32, 32, 32, 32, 180, 360, 10))
    g["K"] = (80, [rect(0, 0, V, 100), line((14, 34), (82, 108), 26), line((34, 60), (88, -10), 26)])
    g["L"] = (60, [rect(0, 0, V, 100), rect(0, 0, 60, H)])
    # the middle vee as two trapezoids meeting on a flat bottom
    g["M"] = (100, [rect(0, 0, V, 100), rect(78, 0, 100, 100), [(0, 100), (24, 100), (60, 30), (40, 30)], [(76, 100), (100, 100), (60, 30), (40, 30)]])
    g["N"] = (82, [rect(0, 0, V, 100), rect(60, 0, 82, 100), line((8, 106), (74, -6), 26)])
    g["O"] = (88, ring(44, 50, 44, 50, 0, 360, 24))
    g["P"] = (74, [rect(0, 0, V, 100)] + bowl(V, 100, 36, 74))
    g["Q"] = (90, ring(44, 50, 44, 50, 0, 360, 24) + [line((64, 20), (98, -8), 24)])
    g["R"] = (78, [rect(0, 0, V, 100)] + bowl(V, 100, 38, 72) + [line((36, 52), (86, -10), 26)])
    g["S"] = (74, ring(37, 70.5, 35, 29.5, 18, 270, 12) + ring(37, 29.5, 37, 29.5, 90, -162, 12))
    g["T"] = (78, [rect(0, 100 - H, 78, 100), rect(28, 0, 50, 100)])
    g["U"] = (80, [rect(0, 38, V, 100), rect(58, 38, 80, 100)] + ring(40, 40, 40, 40, 180, 360, 12))
    g["V"] = (88, [[(0, 100), (24, 100), (56, 0), (32, 0)], [(64, 100), (88, 100), (56, 0), (32, 0)]])
    # four legs meeting on flat feet and a flat middle top
    g["W"] = (114, [[(0, 100), (22, 100), (46, 0), (24, 0)], [(24, 0), (46, 0), (68, 80), (46, 80)], [(46, 80), (68, 80), (90, 0), (68, 0)], [(68, 0), (90, 0), (114, 100), (92, 100)]])
    g["X"] = (86, [line((8, 108), (78, -8), 26), line((78, 108), (8, -8), 26)])
    g["Y"] = (86, [line((6, 106), (43, 46), 26), line((80, 106), (43, 46), 26), rect(32, 0, 54, 54)])
    g["Z"] = (72, [rect(0, 100 - H, 72, 100), rect(0, 0, 72, H), [(48, 100 - H), (72, 100 - H), (24, H), (0, H)]])
    # digits
    g["0"] = (70, ring(35, 50, 35, 50, 0, 360, 24, tx=20))
    g["1"] = (50, [rect(22, 0, 44, 100), line((4, 76), (36, 100), 22)])
    # the hook's arc, then a stroke from its end down to the foot
    two = ring(35, 66, 34, 34, 165, -38, 10)
    end = two[-1]
    g["2"] = (70, two + [[end[1], end[2], (0, H), (28, H)], rect(0, 0, 70, H)])
    g["3"] = (68, ring(32, 70.5, 34, 29.5, 160, -90, 12) + ring(32, 29.5, 36, 29.5, 90, -160, 12))
    g["4"] = (76, [rect(46, 0, 68, 100), rect(0, 22, 76, 22 + H), [(46, 100), (68, 100), (22, 40), (0, 40)]])
    g["5"] = (70, [rect(6, 100 - H, 66, 100), rect(6, 46, 28, 100)] + ring(34, 32, 36, 32, 132, -155, 12))
    six = ring(36, 32, 36, 32, 0, 360, 20) + ring(38, 50, 38, 50, 55, 200, 10)
    g["6"] = (72, six)
    g["7"] = (68, [rect(0, 100 - H, 68, 100), [(44, 100 - H), (68, 100 - H), (34, 0), (10, 0)]])
    g["8"] = (72, ring(36, 71.5, 32, 28.5, 0, 360, 20, tx=20, ty=17) + ring(36, 29.5, 36, 29.5, 0, 360, 20))
    g["9"] = (72, flip(six, 72))
    return g


def main():
    out = {}
    for key, (w, polys) in sorted(glyphs().items()):
        pieces = []
        for k, p in enumerate(polys):
            t = tidy(p, w)
            if len(t) < 3:
                continue
            if not convex(t):
                raise SystemExit(f"{key}: piece {k} is not convex: {t}")
            pieces.append([list(q) for q in t])
        out[key] = {"width": w, "polys": pieces}
    note = (
        "Every capital letter and digit as convex polygons, a chunky rounded sans for the tiles "
        "(crates/core/src/glyph.rs). Each glyph's grid: x from 0 to its width, y from 0 (the baseline) "
        "to height. A hole is the space the pieces round it leave; pieces may overlap. gap: the space "
        "between two glyphs in a text; lift_um: how far off a tile's face the text floats. First drawn "
        "by tools/glyphs/make_glyphs.py; this file is the source of truth."
    )
    doc = {"_note": note, "height": HEIGHT, "gap": GAP, "lift_um": LIFT_UM}
    lines = ["{"]
    for k, v in doc.items():
        lines.append(f"  {json.dumps(k)}: {json.dumps(v)},")
    lines.append('  "glyphs": {')
    items = list(out.items())
    for i, (k, v) in enumerate(items):
        comma = "," if i + 1 < len(items) else ""
        lines.append(f"    {json.dumps(k)}: {json.dumps(v, separators=(',', ':'))}{comma}")
    lines.append("  }")
    lines.append("}")
    (ROOT / "data" / "glyphs.json").write_text("\n".join(lines) + "\n")
    print(f"data/glyphs.json: {len(out)} glyphs, {sum(len(v['polys']) for v in out.values())} pieces")


if __name__ == "__main__":
    main()
