#!/usr/bin/env python3
"""Writes the seafloor scenery round the outside of the map (F14): kelp
that sways, sea anemones, three corals, a school of small fish swimming a
loop, a sunken anchor, a bottle and soft light shafts from the surface.

    python3 tools/models/scenery.py

Like tools/models/world.py (whose Mesh and file writer it uses), the JSON
files under data/models/ are the source of truth and this script drew
their first versions. tools/piles.py places them.

Model rules (CLAUDE.md): no boxes; stems, tentacles and branches are
tapered 6-8 sided tubes, bodies and domes rounded forms of 10+ sides.
Real creatures follow their real body plans:

- kelp (giant kelp): a holdfast of root-like haptera on the sand, long
  stipes rising from it, a blade on each stipe node with a gas bladder
  at its base, the blades longer toward the top;
- sea anemone: a squat column on a wider foot, an oral disc on top with
  the mouth in its middle, and two rings of tapering tentacles round
  the disc's rim (the outer ring longer);
- staghorn coral: a short trunk forking again and again into round,
  tapering branches with pale growing tips; brain coral: a low dome
  grooved in winding ridges; sea fan: a flat fan of thin branching
  ribs on one short stem;
- fish (a small silvery reef fish): a body laterally flattened (taller
  than wide), widest about a third back from the snout, eyes on the
  sides of the head, a dorsal fin on the back, pectoral fins behind the
  gills, pelvic and anal fins below, a narrow tail stalk and a forked
  tail fin standing upright.
"""
import math
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from world import Mesh, hash01, model, write  # noqa: E402


# ---------------------------------------------------------------- helpers


def sub(a, b):
    return [a[i] - b[i] for i in range(3)]


def add(a, b):
    return [a[i] + b[i] for i in range(3)]


def mul(a, s):
    return [a[i] * s for i in range(3)]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def unit(a):
    n = math.sqrt(sum(x * x for x in a)) or 1.0
    return [x / n for x in a]


def frame(t):
    """Two unit vectors at right angles to the direction t and each other."""
    t = unit(t)
    ref = [0, 1, 0] if abs(t[1]) < 0.9 else [1, 0, 0]
    n = unit(cross(t, ref))
    return n, cross(t, n)


def ring(m, centre, t, r, sides, twist=0.0, squash=1.0):
    n, b = frame(t)
    out = []
    for k in range(sides):
        a = 2 * math.pi * k / sides + twist
        p = add(centre, add(mul(n, r * math.cos(a)), mul(b, r * squash * math.sin(a))))
        out.append(m.v(p))
    return out


def tube(m, pts, radii, sides, colour, tip=True, base=False, colour_fn=None):
    """A tapered tube along the polyline pts (radius by point), in the
    latest part; capped with a point at the end when tip, and with a flat
    face at the start when base. colour_fn(segment, side) picks a colour."""
    rings = []
    for i, p in enumerate(pts):
        if i == 0:
            t = sub(pts[1], pts[0])
        elif i == len(pts) - 1:
            t = sub(pts[-1], pts[-2])
        else:
            t = sub(pts[i + 1], pts[i - 1])
        if tip and i == len(pts) - 1:
            break
        rings.append((ring(m, p, t, radii[i], sides), p))
    for i in range(len(rings) - 1):
        (a, pa), (b, pb) = rings[i], rings[i + 1]
        mid = mul(add(pa, pb), 0.5)
        for k in range(sides):
            k1 = (k + 1) % sides
            c = colour_fn(i, k) if colour_fn else colour
            m.face(c, [a[k], a[k1], b[k1], b[k]], away=mid)
    if tip:
        last, pl = rings[-1]
        end = m.v(pts[-1])
        mid = mul(add(pl, pts[-1]), 0.5)
        for k in range(sides):
            c = colour_fn(len(rings) - 1, k) if colour_fn else colour
            m.face(c, [last[k], last[(k + 1) % sides], end], away=mid)
    if base:
        first, p0 = rings[0]
        m.face(colour, first, away=pts[1])
    return rings


def dome(m, rx, ry, rz, sides, bands, colour_fn, sink=0.0, seed=0, jitter=0.0):
    """A dome over the sand, its rim at y = -sink; colour_fn(band, side)."""
    rows = []
    for b in range(bands):
        lat = math.pi / 2 * b / bands
        row = []
        for k in range(sides):
            a = 2 * math.pi * k / sides
            j = 1 + (hash01(seed, b, k) - 0.5) * jitter
            row.append(m.v((rx * math.cos(lat) * math.cos(a) * j, ry * math.sin(lat) - sink,
                            rz * math.cos(lat) * math.sin(a) * j)))
        rows.append(row)
    top = m.v((0, ry - sink, 0))
    centre = (0, ry * 0.3, 0)
    for b in range(bands - 1):
        for k in range(sides):
            k1 = (k + 1) % sides
            m.face(colour_fn(b, k), [rows[b][k], rows[b][k1], rows[b + 1][k1], rows[b + 1][k]], away=centre)
    for k in range(sides):
        m.face(colour_fn(bands - 1, k), [rows[-1][k], rows[-1][(k + 1) % sides], top], away=centre)
    return rows


def sway_keys(names, amp, period, phase, steps=3):
    """Three keys of a loop that tips each named part about x and z, more
    up the chain (amp[i] degrees for names[i]), phase in radians."""
    keys = []
    for s in range(steps):
        q = 2 * math.pi * s / steps
        parts = {}
        for i, name in enumerate(names):
            p = phase[i] if isinstance(phase, list) else phase
            parts[name] = {"turn": [round(amp[i] * math.sin(q + p)), 0, round(amp[i] * math.cos(q + p) * 0.7)]}
        keys.append({"at": round(period * s / steps), "parts": parts})
    return keys


def merge_keys(*lists):
    """Joins key lists with the same times."""
    out = []
    for group in zip(*lists):
        parts = {}
        for k in group:
            assert k["at"] == group[0]["at"]
            parts.update(k["parts"])
        out.append({"at": group[0]["at"], "parts": parts})
    return out


# ---------------------------------------------------------------- kelp


def kelp():
    m = Mesh()
    m.part("holdfast")
    # the holdfast: a low knot of root-like haptera spreading on the sand
    for h in range(7):
        a = 2 * math.pi * h / 7 + hash01(h, 1) * 0.5
        reach = 120 + hash01(h, 2) * 60
        pts = [[0, 50, 0], [math.cos(a) * reach * 0.55, 34, math.sin(a) * reach * 0.55],
               [math.cos(a) * reach, 4, math.sin(a) * reach]]
        tube(m, pts, [24, 16, 9], 6, "holdfast")
    m.face("holdfast", ring(m, [0, 62, 0], [0, 1, 0], 30, 6), up=True)
    stipes = 3
    segs = 5
    keys = []
    for s in range(stipes):
        a = 2 * math.pi * s / stipes + 0.4
        lean = 0.10 + hash01(s, 3) * 0.08
        height = 1700 + hash01(s, 4) * 600
        prev = "holdfast"
        names = []
        base = [math.cos(a) * 18, 55, math.sin(a) * 18]
        for g in range(segs):
            name = f"stipe{s}_{g}"
            names.append(name)
            y0 = base[1] + height * g / segs
            y1 = base[1] + height * (g + 1) / segs
            off0 = 18 + lean * (y0 - 55)
            off1 = 18 + lean * (y1 - 55)
            p0 = [math.cos(a) * off0, y0, math.sin(a) * off0]
            p1 = [math.cos(a) * off1, y1, math.sin(a) * off1]
            m.part(name, parent=prev, pivot=[round(c) for c in p0])
            r0 = 14 - 8 * g / segs
            r1 = 14 - 8 * (g + 1) / segs
            tube(m, [p0, p1], [r0, r1], 6, "stipe", tip=(g == segs - 1))
            # two blades a segment, at its middle and at its top node, on
            # alternate sides, longer up the stipe, each rising out and up
            # from a small gas bladder
            for half in (0, 1):
                side = 1 if (2 * g + half + s) % 2 else -1
                ba = a + side * (1.3 + hash01(s, g, half, 5) * 0.5)
                node = mul(add(p0, p1), 0.5) if half == 0 else (p1 if g < segs - 1 else mul(add(p0, mul(p1, 3)), 0.25))
                dirx, dirz = math.cos(ba), math.sin(ba)
                bl = 380 + 60 * (g + half / 2) + hash01(s, g, half, 6) * 80
                bw = 110 + 8 * g
                bladder = [node[0] + dirx * 24, node[1] + 6, node[2] + dirz * 24]
                # the bladder: a small rounded bead (six sides, two points)
                pr = ring(m, bladder, [dirx, 0.3, dirz], 13, 6)
                pa = m.v(add(bladder, [dirx * 18, 5, dirz * 18]))
                pb = m.v(add(bladder, [-dirx * 14, -4, -dirz * 14]))
                for k in range(6):
                    m.face("bladder", [pr[k], pr[(k + 1) % 6], pa], away=bladder)
                    m.face("bladder", [pr[k], pr[(k + 1) % 6], pb], away=bladder)
                # the blade: a long ribbed leaf, flat across, rising at a slant
                # and curving over toward its tip
                root = [bladder[0] + dirx * 18, bladder[1] + 4, bladder[2] + dirz * 18]
                wx, wz = -dirz, dirx
                pts = []
                for f, wf, up in ((0.0, 0.25, 0.0), (0.25, 1.0, 0.7), (0.6, 0.85, 0.5), (1.0, 0.0, 0.25)):
                    c = [root[0] + dirx * bl * f * 0.8, root[1] + bl * f * up, root[2] + dirz * bl * f * 0.8]
                    w = bw * wf / 2
                    if wf == 0:
                        tipv = m.v(c)
                        pts.append((tipv, tipv))
                    else:
                        pts.append((m.v([c[0] + wx * w, c[1], c[2] + wz * w]), m.v([c[0] - wx * w, c[1] - 6, c[2] - wz * w])))
                for i in range(3):
                    col = "blade" if (i + half) % 2 == 0 else "blade_light"
                    (a0, a1), (b0, b1) = pts[i], pts[i + 1]
                    m.face(col, [a0, a1, b1, b0] if i < 2 else [a0, a1, b0], up=True)
            prev = name
        amp = [3 + 2.5 * g for g in range(segs)]
        keys.append(sway_keys(names, amp, 4800, hash01(s, 9) * 2 * math.pi))
    anim = {"sway": {"ms": 4800, "ease": "smooth", "keys": merge_keys(*keys)}}
    return model(
        "Giant kelp, 1.8 to 2.4 m tall: a holdfast of seven root-like haptera knotted on the sand, three tapering "
        "six-sided stipes rising from it, each in five segments; two long ribbed blades a segment lean out and up "
        "from round gas bladders, on alternate sides, longer toward the top. The stipes sway in the current, more "
        "toward the top (sway, 4.8 s round). Written by tools/models/scenery.py.",
        "Kelp", "prop",
        {"holdfast": [112, 96, 56], "stipe": [128, 112, 52], "bladder": [150, 132, 60],
         "blade": [120, 136, 58], "blade_light": [146, 160, 72]},
        m, view={"yaw": 20, "pitch": 12}, animations=anim)


# ---------------------------------------------------------------- anemone


def anemone():
    m = Mesh()
    m.part("column")
    sides = 12
    # the foot, wider, on the sand; the column; the disc's rim
    stations = [(0, 92), (14, 84), (60, 70), (110, 74), (138, 84)]
    rings = [ring(m, [0, y, 0], [0, 1, 0], r, sides) for y, r in stations]
    for i in range(len(rings) - 1):
        for k in range(sides):
            k1 = (k + 1) % sides
            col = "column_dark" if i == 0 else ("column" if k % 2 else "column_light")
            m.face(col, [rings[i][k], rings[i][k1], rings[i + 1][k1], rings[i + 1][k]],
                   away=(0, (stations[i][0] + stations[i + 1][0]) / 2, 0))
    # the oral disc, dipping to the mouth in the middle
    inner = ring(m, [0, 128, 0], [0, 1, 0], 40, sides)
    mouth = m.v((0, 120, 0))
    for k in range(sides):
        k1 = (k + 1) % sides
        m.face("disc", [rings[-1][k], rings[-1][k1], inner[k1], inner[k]], up=True)
        m.face("mouth", [inner[k], inner[k1], mouth], up=True)
    keys = []
    names = []
    phases = []
    # two rings of tapering tentacles round the rim, the outer longer
    for ring_i, (count, rr, length, rad, up) in enumerate(((14, 74, 130, 17, 0.6), (9, 48, 95, 14, 0.85))):
        for t in range(count):
            a = 2 * math.pi * (t + 0.5 * ring_i) / count
            ca, sa = math.cos(a), math.sin(a)
            name = f"tentacle{ring_i}_{t}"
            names.append(name)
            phases.append(a + ring_i)
            p0 = [ca * rr, 134, sa * rr]
            m.part(name, parent="column", pivot=[round(c) for c in p0])
            out = math.sqrt(1 - up * up)
            # curving out, then up at the tip
            p1 = [p0[0] + ca * out * length * 0.5, p0[1] + up * length * 0.5, p0[2] + sa * out * length * 0.5]
            p2 = [p1[0] + ca * out * length * 0.35, p1[1] + (up + 0.2) * length * 0.4, p1[2] + sa * out * length * 0.35]
            p3 = add(p2, mul(unit(sub(p2, p1)), rad * 0.9))
            tube(m, [p0, p1, p2, p3], [rad, rad * 0.85, rad * 0.6, 0], 6, "tentacle",
                 colour_fn=lambda i, k: "tentacle_tip" if i >= 2 else "tentacle")
    amps = [9 if n.startswith("tentacle0") else 6 for n in names]
    keys = sway_keys(names, amps, 3000, phases)
    anim = {"sway": {"ms": 3000, "ease": "smooth", "keys": keys}}
    return model(
        "A sea anemone about 380 mm across and 300 mm tall: a squat twelve-sided column on a wider foot, a flat oral "
        "disc on top dipping to the mouth in its middle, and two rings of thick, blunt, tapering six-sided tentacles round the "
        "disc's rim, fourteen long ones outside and nine shorter inside, curving out then up, pale at the tips. "
        "The tentacles wave in the current (sway, 3 s round). Written by tools/models/scenery.py.",
        "Sea anemone", "prop",
        {"column": [196, 96, 120], "column_light": [214, 120, 140], "column_dark": [160, 76, 100],
         "disc": [226, 150, 160], "mouth": [140, 60, 86], "tentacle": [236, 140, 170], "tentacle_tip": [250, 206, 214]},
        m, view={"yaw": 30, "pitch": 30}, animations=anim)


# ---------------------------------------------------------------- corals


def staghorn():
    m = Mesh()
    m.part("coral")
    count = [0]

    def branch(p, d, length, r, depth, seed):
        d = unit(d)
        bend = [d[0] * 0.2, 0.25, d[2] * 0.2]
        mid = add(p, mul(d, length * 0.5))
        end = add(add(p, mul(d, length)), mul(bend, length * 0.15))
        tipped = depth == 0
        tube(m, [p, mid, end] + ([add(end, mul(unit(sub(end, mid)), r * 1.3))] if tipped else []),
             [r, r * 0.85, r * 0.72] + ([0] if tipped else []), 6, "coral",
             tip=tipped, colour_fn=(lambda i, k: "coral_tip" if tipped and i >= 1 else "coral"))
        count[0] += 1
        if depth == 0:
            return
        n = 2 if depth > 1 else 2 + (seed % 2)
        for k in range(n):
            a = 2 * math.pi * (k / n + hash01(seed, k, 1) * 0.3)
            out = 0.55 + hash01(seed, k, 2) * 0.3
            nd = unit([d[0] * 0.6 + math.cos(a) * out, d[1] * 0.8 + 0.25, d[2] * 0.6 + math.sin(a) * out])
            branch(end, nd, length * (0.68 + hash01(seed, k, 3) * 0.15), r * 0.72, depth - 1, seed * 7 + k + 1)

    for k in range(5):
        a = 2 * math.pi * k / 5 + 0.3
        lean = 0.35 + hash01(k, 8) * 0.3
        branch([math.cos(a) * 60, 0, math.sin(a) * 60], [math.cos(a) * lean, 1, math.sin(a) * lean], 170 + hash01(k, 9) * 60, 40, 3, k + 2)
    return model(
        "A staghorn coral about 1 m across and 0.6 m tall: five short trunks from the sand forking again and "
        "again into round, tapering six-sided branches that curve upward, with pale growing tips. "
        "Written by tools/models/scenery.py.",
        "Staghorn coral", "prop", {"coral": [255, 136, 112], "coral_tip": [255, 222, 196]},
        m, view={"yaw": 25, "pitch": 20})


def brain_coral():
    m = Mesh()
    m.part("coral")

    # winding ridges: a colour by a wavy band across the dome
    def col(b, k):
        a = 2 * math.pi * k / 24
        wave = math.sin(3 * a + b * 1.1) + math.sin(5 * a - b * 0.6)
        return "brain_ridge" if (b + round(wave * 1.5)) % 2 == 0 else "brain_groove"

    dome(m, 420, 300, 380, 24, 9, col, sink=40, seed=21, jitter=0.06)
    return model(
        "A brain coral, a low dome about 840 mm across and 260 mm over the sand, sunk a little: twenty-four sides "
        "and nine bands, coloured in winding ridges and grooves. Written by tools/models/scenery.py.",
        "Brain coral", "prop", {"brain_ridge": [190, 194, 108], "brain_groove": [126, 136, 74]},
        m, view={"yaw": 25, "pitch": 35})


def sea_fan():
    m = Mesh()
    m.part("fan")
    # a short stem, then ribs fanning out in the x-y plane, forking
    stem_top = [0, 160, 0]
    tube(m, [[0, 0, 0], [0, 80, 0], stem_top], [26, 20, 16], 6, "fan_stem", tip=False)

    def rib(p, a, length, r, depth, seed):
        d = [math.cos(a), math.sin(a), 0]
        end = add(p, mul(d, length))
        sway = [0, 0, (hash01(seed, 4) - 0.5) * 30]
        end = add(end, sway)
        tube(m, [p, end] + ([add(end, mul(d, 20))] if depth == 0 else []),
             [r, r * 0.8] + ([0] if depth == 0 else []), 6, "fan", tip=depth == 0)
        if depth == 0:
            return
        for k, da in enumerate((-0.32, 0.32)):
            rib(end, a + da + (hash01(seed, k) - 0.5) * 0.15, length * 0.78, r * 0.78, depth - 1, seed * 3 + k + 1)

    for k, a in enumerate((math.radians(55), math.radians(90), math.radians(125))):
        rib(stem_top, a, 260, 14, 3, k + 5)
    # the fine net between the ribs: one flat fan just behind them, its
    # edge wavy, a shade paler than the ribs
    edge = []
    for k in range(13):
        a = math.radians(40 + 100 * k / 12)
        r = 600 + 45 * math.sin(k * 1.7)
        edge.append(m.v([math.cos(a) * r, stem_top[1] + math.sin(a) * r, -4]))
    m.face("fan_net", [m.v([0, stem_top[1] + 20, -4])] + edge)
    return model(
        "A sea fan (a gorgonian) about 1.1 m across and 1 m tall: a short six-sided stem and, from its top, three "
        "ribs fanning out in one flat plane, each forking three times into thinner tapering ribs, the fine net "
        "between them a flat fan behind. "
        "Written by tools/models/scenery.py.",
        "Sea fan", "prop", {"fan_stem": [120, 64, 110], "fan": [168, 92, 168], "fan_net": [190, 120, 186]},
        m, view={"yaw": 10, "pitch": 10})


# ---------------------------------------------------------------- fish

FISH_LEN = 190


def fish_body(m, at, scale, prefix):
    """One fish, nose toward -z, its middle at `at`; returns its tail's
    part name. The body is part `prefix`, the tail fin part `prefix_tail`."""
    sx = scale
    m.part(prefix, parent="school", pivot=[round(c) for c in at])
    # stations from the snout back: (z, half height, half width); widest a
    # third back from the snout, flattened side to side
    stations = [(-95, 0, 0), (-82, 17, 10), (-58, 31, 16), (-30, 36, 18), (2, 32, 15), (32, 22, 10), (56, 12, 6), (70, 8, 4)]
    sides = 10
    rings = []
    for z, hh, hw in stations[1:]:
        row = []
        for k in range(sides):
            a = 2 * math.pi * k / sides
            row.append(m.v(add(at, [hw * math.cos(a) * sx, hh * math.sin(a) * sx, z * sx])))
        rings.append(row)
    snout = m.v(add(at, [0, -2 * sx, -95 * sx]))

    def col(k, i):
        y = math.sin(2 * math.pi * (k + 0.5) / sides)
        if y > 0.45:
            return "fish_back"
        if y < -0.45:
            return "fish_belly"
        # a stripe along each flank, just over the middle, gill to tail
        return "fish_stripe" if 0 < y and 1 <= i <= 5 else "fish_side"

    centre = add(at, [0, 0, -10 * sx])
    for k in range(sides):
        k1 = (k + 1) % sides
        m.face(col(k, 0), [snout, rings[0][k], rings[0][k1]], away=centre)
    for i in range(len(rings) - 1):
        for k in range(sides):
            k1 = (k + 1) % sides
            m.face(col(k, i + 1), [rings[i][k], rings[i][k1], rings[i + 1][k1], rings[i + 1][k]], away=centre)
    m.face("fish_side", rings[-1], away=centre)

    def p(x, y, z):
        return m.v(add(at, [x * sx, y * sx, z * sx]))

    # the eyes on the sides of the head, standing just proud
    for s in (1, -1):
        eye = [p(s * 13, 10 + 6 * math.sin(2 * math.pi * k / 6), -68 + 6 * math.cos(2 * math.pi * k / 6)) for k in range(6)]
        m.face("fish_eye", eye, away=add(at, [0, 10 * sx, -68 * sx]))
    # the dorsal fin along the back, swept back
    m.face("fish_fin", [p(0, 34, -40), p(0, 58, -8), p(0, 46, 14), p(0, 27, 26)], away=add(at, [10, 40 * sx, 0]))
    # the pectoral fins behind the gills, angled back and out
    for s in (1, -1):
        m.face("fish_fin", [p(s * 15, -4, -46), p(s * 34, -14, -24), p(s * 18, -12, -30)], away=add(at, [0, 30 * sx, -36 * sx]))
    # the pelvic fins below, under the pectorals, and the anal fin behind
    for s in (1, -1):
        m.face("fish_fin", [p(s * 5, -30, -30), p(s * 10, -46, -16), p(s * 4, -32, -16)], away=add(at, [0, 0, -24 * sx]))
    m.face("fish_fin", [p(0, -22, 22), p(0, -38, 42), p(0, -14, 50)], away=add(at, [10, 0, 36 * sx]))
    # the tail fin: forked, upright, on its own part so it beats
    tail = f"{prefix}_tail"
    m.part(tail, parent=prefix, pivot=[round(c) for c in add(at, [0, 0, 68 * sx])])
    a0, a1 = p(0, 7, 68), p(0, -7, 68)
    up_tip, lo_tip, notch = p(0, 42, 112), p(0, -40, 110), p(0, 0, 92)
    m.face("fish_tail", [a0, up_tip, notch], away=add(at, [10, 0, 90 * sx]))
    m.face("fish_tail", [a1, notch, lo_tip], away=add(at, [10, 0, 90 * sx]))
    m.face("fish_tail", [a0, notch, a1], away=add(at, [10, 0, 90 * sx]))
    return prefix, tail


def fish_school():
    """Six small fish swimming a loop 1.6 m round the model's middle, about
    0.9 m over the sand, the root part turning a full turn each round; each
    fish's tail beats and its body wags a little against it."""
    m = Mesh()
    m.part("school")
    # the loop's middle: a speck under the sand (a part needs a face)
    m.face("fish_fin", [m.v((0, -20, 0)), m.v((10, -20, 0)), m.v((0, -20, 10))])
    radius = 1600
    # where each fish swims in the school, (out from the loop, up, along)
    spots = [(0, 900, 0), (140, 980, 220), (-130, 860, 260), (60, 1040, 480), (-60, 900, 520), (160, 860, -260)]
    fish = []
    for i, (dr, dy, dz) in enumerate(spots):
        s = 0.85 + hash01(i, 11) * 0.3
        fish.append(fish_body(m, [radius + dr, dy, dz], s, f"fish{i}"))
    ms = 24000
    step = 300
    beat = 1200  # a tail beat, out and back: keys at its quarters
    keys = []
    for at in list(range(0, ms - step, step)) + [ms - 1]:
        parts = {"school": {"turn": [0, round(360 * at / ms), 0], "move": [0, round(40 * math.sin(2 * math.pi * at / 6000)), 0]}}
        for i, (body, tail) in enumerate(fish):
            q = 2 * math.pi * (at / beat + i * 0.25)
            parts[tail] = {"turn": [0, round(22 * math.sin(q)), 0]}
            parts[body] = {"turn": [0, round(-5 * math.sin(q)), 0]}
        keys.append({"at": at, "parts": parts})
    anim = {"sway": {"ms": ms, "ease": "linear", "keys": keys}}
    return model(
        "A school of six small silvery reef fish, each about 190 mm long: a body flattened side to side and widest a "
        "third back from the snout, ten-sided, blue-grey on the back with a yellow stripe along the flank and a pale "
        "belly; an eye on each side of the head, a swept dorsal fin, pectoral fins behind the gills, pelvic and anal "
        "fins below and a forked yellow tail. They swim nose first round a loop 3.2 m across about 0.9 m over the sand, "
        "once every 24 s, tails beating (sway). The middle of the model is the loop's middle. "
        "Written by tools/models/scenery.py.",
        "Fish", "prop",
        {"fish_back": [84, 120, 156], "fish_side": [196, 210, 214], "fish_stripe": [240, 200, 92], "fish_belly": [236, 238, 230],
         "fish_fin": [212, 196, 140], "fish_tail": [240, 196, 84], "fish_eye": [36, 40, 52]},
        m, view={"yaw": 60, "pitch": 20}, animations=anim)


# ---------------------------------------------------------------- anchor and bottle


def arc(c, r, a0, a1, n, y0=0.0):
    """Points round an arc in the x-y plane about c."""
    return [[c[0] + r * math.cos(a0 + (a1 - a0) * i / n), c[1] + r * math.sin(a0 + (a1 - a0) * i / n) + y0, c[2]] for i in range(n + 1)]


def anchor():
    """An old stocked anchor, built standing (shank up the y axis, crown at
    the bottom, arms in the x-y plane, stock across in z); the scene lays
    it over on the sand."""
    m = Mesh()
    m.part("anchor")
    rust = lambda i, k: "rust_dark" if (i + k) % 3 == 0 else "rust"
    # the shank, tapering up from the crown
    tube(m, [[0, 0, 0], [0, 420, 0], [0, 860, 0]], [34, 28, 24], 8, "rust", tip=False, base=True, colour_fn=rust)
    m.face("rust_dark", ring(m, [0, 860, 0], [0, 1, 0], 24, 8), up=True)
    # the ring at the top: a torus of ten segments
    rc, rr = [0, 960, 0], 80
    pts = arc(rc, rr, -math.pi / 2 - 0.3, 3 * math.pi / 2 + 0.3, 10)
    tube(m, pts, [14] * len(pts), 6, "rust", tip=False, colour_fn=rust)
    # the stock: a bar across near the top, thicker in the middle
    tube(m, [[0, 760, -420], [0, 760, -200], [0, 760, 0], [0, 760, 200], [0, 760, 420]], [14, 20, 26, 20, 14], 8, "rust",
         tip=False, base=True, colour_fn=rust)
    # the arms: curving up from the crown, each ending in a spade fluke
    for s in (1, -1):
        pts = arc([0, 330, 0], 330, -math.pi / 2, -math.pi / 2 + s * math.radians(80), 5)
        tube(m, pts, [32, 30, 27, 24, 21, 18], 8, "rust", tip=False, colour_fn=rust)
        end = pts[-1]
        d = unit(sub(pts[-1], pts[-2]))
        across = [0, 0, 1]
        base = sub(end, mul(d, 30))
        tipp = add(end, mul(d, 150))
        left = add(add(end, mul(across, 90)), mul(d, 10))
        right = add(add(end, mul(across, -90)), mul(d, 10))
        n = unit(cross(d, across))
        bulge = mul(n, 14)
        vb, vt, vl, vr = m.v(base), m.v(tipp), m.v(left), m.v(right)
        f1, f2 = m.v(add(end, bulge)), m.v(sub(end, bulge))
        for a, b in ((vb, vl), (vl, vt), (vt, vr), (vr, vb)):
            m.face("rust_light", [a, b, f1], away=sub(end, mul(n, 40)))
            m.face("rust_dark", [a, b, f2], away=add(end, mul(n, 40)))
    # the crown: a rounded knot where the arms meet the shank
    m.part("crown", parent="anchor")
    dome(m, 48, 40, 48, 8, 3, lambda b, k: "rust_dark", sink=10)
    return model(
        "An old sunken anchor about 1.2 m long: an eight-sided shank tapering up from a rounded crown, two arms curving up "
        "from the crown, each ending in a spade-shaped fluke, a stock across near the top, and a ring at the top; rusty "
        "brown. Built standing; the map lays it over, half in the sand. Written by tools/models/scenery.py.",
        "Anchor", "prop", {"rust": [146, 92, 66], "rust_light": [172, 116, 80], "rust_dark": [108, 70, 56]},
        m, view={"yaw": 70, "pitch": 15})


def bottle():
    """A glass bottle lying on its side (axis along x, neck toward +x),
    sunk a third into the sand, with a cork."""
    m = Mesh()
    m.part("bottle")
    sides = 12
    profile = [(-170, 0), (-168, 52), (-150, 60), (40, 60), (80, 48), (110, 28), (150, 22), (170, 24), (176, 20)]
    sink = 22
    rings = []
    for x, r in profile:
        if r == 0:
            rings.append([m.v((x, 60 - sink, 0))])
            continue
        rings.append(ring(m, [x, 60 - sink, 0], [1, 0, 0], r, sides))
    for i in range(len(rings) - 1):
        a, b = rings[i], rings[i + 1]
        mid = ((profile[i][0] + profile[i + 1][0]) / 2, 60 - sink, 0)
        for k in range(sides):
            k1 = (k + 1) % sides
            col = "glass_light" if k in (2, 3) and 1 < i < 4 else "glass"
            if len(a) == 1:
                m.face("glass_dark", [a[0], b[k], b[k1]], away=mid)
            else:
                m.face(col, [a[k], a[k1], b[k1], b[k]], away=mid)
    # the cork in the mouth
    cork = tube(m, [[170, 60 - sink, 0], [206, 60 - sink, 0]], [18, 17], 8, "cork", tip=False)
    m.face("cork", cork[-1][0], away=(150, 60 - sink, 0))
    return model(
        "A glass bottle about 380 mm long lying on its side, sunk a third into the sand: a twelve-sided round body, "
        "a sloping shoulder and a narrow neck with a lip, sea-green glass with a paler shine along it, and a cork "
        "in the mouth. Written by tools/models/scenery.py.",
        "Bottle", "prop",
        {"glass": [72, 132, 108], "glass_light": [150, 200, 176], "glass_dark": [52, 100, 84], "cork": [176, 136, 92]},
        m, view={"yaw": 30, "pitch": 25})


# ---------------------------------------------------------------- light shaft


def light_shaft():
    """A soft shaft of sunlight from the surface: a slanting eight-sided
    tube 7 m tall, narrow at the top and spreading toward the sand, see-
    through and unlit, ending a little over the sand. Two nested tubes,
    the inner a little brighter."""
    m = Mesh()
    m.part("shaft")
    top_y, foot_y = 7000, 150
    lean = 1800  # the top is this far toward +x: the sun is off that way
    for r0, r1, col in ((260, 760, "light"), (140, 420, "light_core")):
        top = [lean, top_y, 0]
        foot = [0, foot_y, 0]
        tube(m, [top, mul(add(top, foot), 0.5), foot], [r0, (r0 + r1) / 2, r1], 8, col, tip=False)
    return model(
        "A soft shaft of sunlight from the surface: two nested slanting eight-sided tubes 7 m tall, narrow at the top and "
        "spreading toward the sand, see-through and unlit (glow), stopping a little over the sand. Drawn each frame "
        "with the other see-through things. Written by tools/models/scenery.py.",
        "Light shaft", "prop",
        {"light": {"rgb": [222, 246, 236], "glow": True, "alpha": 7}, "light_core": {"rgb": [236, 252, 240], "glow": True, "alpha": 8}},
        m, view={"yaw": 0, "pitch": 5})


def main():
    write("kelp", kelp())
    write("anemone", anemone())
    write("coral-staghorn", staghorn())
    write("coral-brain", brain_coral())
    write("sea-fan", sea_fan())
    write("fish", fish_school())
    write("anchor", anchor())
    write("bottle", bottle())
    write("light-shaft", light_shaft())


if __name__ == "__main__":
    main()
