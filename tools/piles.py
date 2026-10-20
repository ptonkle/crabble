#!/usr/bin/env python3
"""Lays out the seafloor map: the board in the middle, sand mounds round
it, the 294 tiles mostly in messy piles on the mounds and some alone, and
a few props; writes it as scenes (data/scenes/seafloor.json and the
close-up stills' scenes). Deterministic: the same seed gives the same
layout, so it can be regenerated after a change.

    python3 tools/piles.py            seed 1
    python3 tools/piles.py --seed 7   another layout

The piles are dropped tile by tile, a light imitation of physics: each
tile falls at a random spot near its pile's middle, at a random heading,
lands flat on the highest point under it and tips about what it touches,
its middle always going down, until its middle is over what holds it
(the underside never goes into anything) (the drawn sand and mound faces from
tools/models/world.py, and the tiles already down). A rest steeper than
wood holds (about 27 degrees) slides it downhill and it settles again.
So tiles lean on each other at tumbled angles and nothing floats; about
a third land face down. The board's square (row r, column c), both 0 to
14, has its centre at x = (c - 7) * 210, z = (r - 7) * 210.
"""
import argparse
import json
import math
import pathlib
import random
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools" / "models"))
import world  # noqa: E402

TILE = 200
THICK = 45
COUNTS = {"A": 9, "B": 2, "C": 2, "D": 4, "E": 12, "F": 2, "G": 3, "H": 2, "I": 9, "J": 1, "K": 1, "L": 4, "M": 2,
          "N": 6, "O": 8, "P": 2, "Q": 1, "R": 6, "S": 4, "T": 6, "U": 4, "V": 2, "W": 2, "X": 1, "Y": 2, "Z": 1}
SETS = 3
REPOSE = 0.5  # tan of the steepest rest that holds, about 27 degrees (wood on wood)
CORNER = 1850  # the middle of each corner's clear patch (the crabs' first start, kept so the piles stay put)
START = 1645  # the crabs start at (+-START, +-START): the board's half (1575) + start_off_corner_mm (100) / sqrt 2
CLEAR = 900  # nothing within this of a crab's start
BOARD_HALF = 1649 + 80  # the board's half-width and a little room

# mounds: (model, x, z, heading in degrees)
MOUNDS = [
    ("mound-dome", 3500, 300, 10),
    ("mound-long", 2900, 3300, 40),
    ("mound-low", -400, 3700, 0),
    ("mound-dome", -3300, 2600, 70),
    ("mound-long", -3700, -600, 95),
    ("mound-low", -2700, -3400, 20),
    ("mound-dome", 300, -3600, 130),
    ("mound-long", 3100, -2900, 135),
    ("mound-low", 4600, -1400, 60),
]
# piles: (x, z, tiles); most near a mound's top
PILES = [
    (3450, 250, 22), (3360, 2915, 20), (2440, 3685, 18), (-450, 3650, 21), (-3300, 2650, 22),
    (-3750, -350, 20), (-3600, -1000, 17), (-2650, -3350, 21), (250, -3550, 22), (3000, -2850, 20),
    (4550, -1350, 19), (1000, 3000, 22),
]
LOOSE = 50


def rot_x(a):
    c, s = math.cos(a), math.sin(a)
    return [[1, 0, 0], [0, c, -s], [0, s, c]]


def rot_y(a):
    c, s = math.cos(a), math.sin(a)
    return [[c, 0, s], [0, 1, 0], [-s, 0, c]]


def rot_z(a):
    c, s = math.cos(a), math.sin(a)
    return [[c, -s, 0], [s, c, 0], [0, 0, 1]]


def mul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def apply(m, v):
    return [sum(m[i][k] * v[k] for k in range(3)) for i in range(3)]


def euler(r):
    """(about x, about y, about z) in degrees with r = Ry(y) Rx(x) Rz(z), as
    the core's placing turns a thing (crates/core/src/scene.rs)."""
    ax = math.asin(max(-1, min(1, -r[1][2])))
    az = math.atan2(r[1][0], r[1][1])
    ay = math.atan2(r[0][2], r[2][2])
    return [math.degrees(ax), math.degrees(ay), math.degrees(az)]


def turned(t):
    ax, ay, az = (math.radians(v) for v in t)
    return mul(rot_y(ay), mul(rot_x(ax), rot_z(az)))


def align(n):
    """The rotation taking +y to the unit vector n by the shortest way."""
    nx, ny, nz = n
    k = (nz, 0.0, -nx)
    s = math.hypot(k[0], k[2])
    if s < 1e-9:
        return [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
    kx, ky, kz = k[0] / s, 0.0, k[2] / s
    th = math.atan2(s, ny)
    c, sn, v = math.cos(th), math.sin(th), 1 - math.cos(th)
    return [[c + kx * kx * v, kx * ky * v - kz * sn, kx * kz * v + ky * sn],
            [ky * kx * v + kz * sn, c + ky * ky * v, ky * kz * v - kx * sn],
            [kz * kx * v - ky * sn, kz * ky * v + kx * sn, c + kz * kz * v]]


# ---------------------------------------------------------------- the ground


class Ground:
    """The drawn sand and mounds, and the tiles laid so far (a grid of
    their tops, 10 mm cells)."""

    CELL = 10

    def __init__(self):
        self.mounds = []
        for mid, x, z, yaw in MOUNDS:
            _, rx, rz, *_ = world.MOUNDS[mid]
            self.mounds.append((mid, x, z, math.radians(yaw), max(rx, rz) * 1.1))
        self.base_cache = {}
        self.tops = {}

    def base(self, x, z):
        key = (round(x / 5), round(z / 5))
        h = self.base_cache.get(key)
        if h is not None:
            return h
        h = world.FLOOR.height(x, z)
        h = -60 if h is None else h
        for mid, mx, mz, yaw, reach in self.mounds:
            dx, dz = x - mx, z - mz
            if abs(dx) > reach or abs(dz) > reach:
                continue
            c, s = math.cos(yaw), math.sin(yaw)
            lx, lz = c * dx - s * dz, s * dx + c * dz
            mh = world.mound_height(mid, lx, lz)
            if mh is not None and mh > h:
                h = mh
        self.base_cache[key] = h
        return h

    def support(self, x, z):
        h = self.base(x, z)
        t = self.tops.get((int(x // self.CELL), int(z // self.CELL)))
        return h if t is None or t < h else t

    def stamp(self, at, r):
        """Marks the cells under a laid tile with the height of its top."""
        ex, ez = (r[0][0], r[2][0]), (r[0][2], r[2][2])  # its x and z axes, seen from above
        ny = r[1][1]
        det = ex[0] * ez[1] - ez[0] * ex[1]
        reach = 150
        c0 = int((at[0] - reach) // self.CELL)
        c1 = int((at[0] + reach) // self.CELL)
        d0 = int((at[2] - reach) // self.CELL)
        d1 = int((at[2] + reach) // self.CELL)
        for i in range(c0, c1 + 1):
            for j in range(d0, d1 + 1):
                x = (i + 0.5) * self.CELL - at[0]
                z = (j + 0.5) * self.CELL - at[2]
                u = (x * ez[1] - z * ez[0]) / det
                w = (ex[0] * z - ex[1] * x) / det
                if abs(u) > TILE / 2 + 3 or abs(w) > TILE / 2 + 3:
                    continue
                y = at[1] + u * r[1][0] + w * r[1][2] + THICK / ny
                k = (i, j)
                if self.tops.get(k, -1e9) < y:
                    self.tops[k] = y


SAMPLES = [(-96 + 24 * a, -96 + 24 * b) for a in range(9) for b in range(9)]


STEEPEST = 1.3  # tan of the steepest a tile may tip while settling, about 52 degrees


def origin_in_hull(pts, eps=1e-6):
    """Whether (0, 0) is inside or on the convex hull of these (x, z)."""
    if any(math.hypot(x, z) < 1.0 for x, z in pts):
        return True
    angles = sorted(math.atan2(z, x) for x, z in pts)
    gaps = [b - a for a, b in zip(angles, angles[1:])] + [angles[0] + 2 * math.pi - angles[-1]]
    return max(gaps) <= math.pi + eps


def rest(ground, cx, cz, offsets):
    """Where a tile dropped at (cx, cz) heading `yaw` comes to rest. It
    lands flat on the highest point under it, then tips about what it
    touches, its middle always going down, until its middle is over what
    holds it: one point under the middle, a line through it, or a ring of
    points round it. Its underside is the plane y = a + b dx + c dz.
    `offsets` are where its underside's sample points lie from its middle,
    seen from above. Returns (a, b, c, settled); not settled means it
    tipped past STEEPEST."""
    pts = [(dx, dz, ground.support(cx + dx, cz + dz)) for dx, dz in offsets]
    a, b, c = max(h for _, _, h in pts), 0.0, 0.0
    eps = 0.3
    for _ in range(60):
        slack = [a + b * x + c * z - h for x, z, h in pts]
        on = [k for k, s in enumerate(slack) if s < eps]
        xz = [(pts[k][0], pts[k][1]) for k in on]
        if origin_in_hull(xz):
            return a, b, c, True
        # the way to tip: away from the edge of what it touches nearest the middle
        if len(on) == 1:
            px, pz = xz[0]
            d = (px, pz)
            p = xz[0]
        else:
            d = None
            for i in range(len(on)):
                for j in range(i + 1, len(on)):
                    (ix, iz), (jx, jz) = xz[i], xz[j]
                    nx, nz = -(jz - iz), jx - ix
                    s0 = nx * (0 - ix) + nz * (0 - iz)
                    if abs(s0) < 1e-6:
                        continue
                    if all((nx * (kx - ix) + nz * (kz - iz)) * s0 <= 1e-6 for kx, kz in xz):
                        sg = -1 if s0 > 0 else 1
                        d, p = (sg * nx, sg * nz), xz[i]
                        break
                if d:
                    break
            if d is None:
                return a, b, c, True
        ln = math.hypot(*d)
        d = (d[0] / ln, d[1] / ln)
        dp = d[0] * p[0] + d[1] * p[1]
        # how far it can tip before something else touches
        t = None
        for k, (x, z, h) in enumerate(pts):
            if k in on:
                continue
            rate = d[0] * (x - p[0]) + d[1] * (z - p[1])
            if rate < -1e-9:
                tk = -slack[k] / rate
                if t is None or tk < t:
                    t = tk
        # and before it is too steep: |(b, c) + t d| = STEEPEST
        bd = b * d[0] + c * d[1]
        tmax = -bd + math.sqrt(max(0.0, bd * bd - (b * b + c * c - STEEPEST ** 2)))
        if t is None or t >= tmax:
            b, c = b + tmax * d[0], c + tmax * d[1]
            a -= tmax * dp
            return a, b, c, False
        b, c = b + t * d[0], c + t * d[1]
        a -= t * dp
    return a, b, c, True


def lay(ground, rng, cx, cz, slide=True):
    """Drops one tile; returns (at, turn, slope) and marks it laid."""
    yaw = rng.uniform(0, 2 * math.pi)
    flat = rot_y(yaw)

    def pose(b, c):
        n = (-b, 1.0, -c)
        ln = math.sqrt(sum(v * v for v in n))
        return mul(align(tuple(v / ln for v in n)), flat)

    for _ in range(20):
        cx, cz = round(cx), round(cz)
        r = flat
        # tipped, its underside's points move in a little: settle again
        # with them where they now are
        for _ in range(3):
            offsets = [(q[0], q[2]) for q in (apply(r, (u, 0, w)) for u, w in SAMPLES)]
            a, b, c, settled = rest(ground, cx, cz, offsets)
            r = pose(b, c)
        s = math.hypot(b, c)
        if not slide or (settled and s <= REPOSE):
            break
        # too steep to hold: it slides downhill and settles again
        cx -= b / s * 40
        cz -= c / s * 40
    t = [round(v) for v in euler(r)]
    r = turned(t)
    # with whole-degree turns it rests touching what is under it
    lift = -1e9
    for u, w in SAMPLES:
        p = apply(r, (u, 0, w))
        lift = max(lift, ground.support(cx + p[0], cz + p[2]) - p[1])
    at = [round(cx), math.ceil(lift), round(cz)]
    ground.stamp(at, turned(t))
    return at, t, s


def letters(rng):
    bag = [l for l, n in COUNTS.items() for _ in range(n * SETS)]
    rng.shuffle(bag)
    return bag


def clear_of_corners(x, z, room=CLEAR):
    return all(math.hypot(x - sx * CORNER, z - sz * CORNER) > room for sx in (1, -1) for sz in (1, -1))


def off_board(x, z, room=0):
    return abs(x) > BOARD_HALF + room or abs(z) > BOARD_HALF + room


def layout(seed):
    rng = random.Random(seed)
    ground = Ground()
    bag = letters(rng)
    assert len(bag) == 294 and sum(n for *_, n in PILES) + LOOSE == 294
    things = [{"model": "seafloor"}, {"model": "board"}]
    for mid, x, z, yaw in MOUNDS:
        things.append({"model": mid, "at": [x, 0, z], "turn": [0, yaw, 0]})
    tiles = []
    steep = []
    piles = []
    for px, pz, count in PILES:
        mine = []
        for _ in range(count):
            while True:
                dx, dz = rng.gauss(0, 210), rng.gauss(0, 210)
                if math.hypot(dx, dz) < 500:
                    break
            at, t, s = lay(ground, rng, px + dx, pz + dz)
            steep.append(s)
            mine.append(len(tiles))
            tiles.append((at, t))
        piles.append(mine)
    loose = []
    while len(loose) < LOOSE:
        r = rng.uniform(1900, 5600)
        a = rng.uniform(0, 2 * math.pi)
        x, z = r * math.cos(a), r * math.sin(a)
        if not off_board(x, z, 60) or not clear_of_corners(x, z):
            continue
        if any(math.hypot(x - px, z - pz) < 700 for px, pz, _ in PILES):
            continue
        if any(math.hypot(x - tiles[i][0][0], z - tiles[i][0][2]) < 320 for i in loose):
            continue
        at, t, s = lay(ground, rng, x, z, slide=False)
        loose.append(len(tiles))
        tiles.append((at, t))
    for k, (at, t) in enumerate(tiles):
        thing = {"model": "tile", "at": at, "turn": t, "letter": bag[k]}
        if rng.random() < 1 / 3:
            thing["face"] = "down"
        things.append(thing)
    things += props(rng, ground)
    stats = {
        "tiles": len(tiles),
        "face_down": sum(1 for t in things if t.get("face") == "down"),
        "steepest_deg": round(math.degrees(math.atan(max(steep)))),
        "leaning_over_20deg": sum(1 for s in steep if s > math.tan(math.radians(20))),
        "highest_mm": max(t[0][1] for t in tiles),
    }
    return things, piles, loose, tiles, stats


def props(rng, ground):
    """Rocks, shells, a starfish and weed: a few, away from the piles, the
    board and the crabs' corners, standing on the sand."""
    spots = [
        ("boulder", 2500, -1500), ("rock-flat", 2300, -1150), ("pebble", 2700, -1100),
        ("boulder", -2700, 700), ("pebble", -2450, 420), ("pebble", -3000, 1000),
        ("rock-flat", -800, -2500), ("boulder", 5200, 1500), ("pebble", 5000, 1900),
        ("boulder", -4700, -2200), ("rock-flat", 1200, 4700), ("pebble", 1500, 4500),
        ("scallop", 2200, 900), ("scallop", -1100, 2350), ("scallop", -3600, 900),
        ("whelk", 1000, -2350), ("whelk", -2500, -1000),
        ("starfish", 2600, 1300), ("starfish", -1500, -2600),
        ("weed", 3900, 1100), ("weed", 4150, 1300), ("weed", -2900, 450),
        ("weed", -4300, -1500), ("weed", -4500, -1250), ("weed", 800, 5000),
        ("weed", 3500, -3800), ("weed", -1800, 4500),
    ]
    out = []
    for k, (mid, x, z) in enumerate(spots):
        assert off_board(x, z) and clear_of_corners(x, z, 650), (mid, x, z)
        y = round(ground.base(x, z))
        t = {"model": mid, "at": [x, y, z], "turn": [0, rng.randrange(360), 0]}
        if mid == "weed":
            t["anim"] = "sway"
            t["ms"] = rng.randrange(3600)
        out.append(t)
    return out


# ---------------------------------------------------------------- scenes


def scene(name, note, camera, things, fog=None):
    d = {"_note": note, "name": name, "camera": camera}
    if fog:
        d["fog"] = fog
    d["things"] = things
    return d


def near(things, x, z, reach):
    """The seafloor and board, and every other thing within reach."""
    out = []
    for t in things:
        at = t.get("at", [0, 0, 0])
        if t["model"] in ("seafloor", "board") or math.hypot(at[0] - x, at[2] - z) < reach:
            out.append(t)
    return out


def write_scene(sid, d):
    lines = ["{"]
    for k in ("_note", "name", "camera", "fog"):
        if k in d:
            lines.append(f"  {json.dumps(k)}: {json.dumps(d[k])},")
    lines.append('  "things": [')
    lines.append(",\n".join("    " + json.dumps(t) for t in d["things"]))
    lines.append("  ]")
    lines.append("}")
    (ROOT / "data" / "scenes" / f"{sid}.json").write_text("\n".join(lines) + "\n")


def square(r, c):
    return [(c - 7) * 210, 40, (r - 7) * 210]


def board_tiles():
    """A few words on the board: SHELL across row 7 through the centre,
    SAND down column 5 from its S, CLAW down column 9 through its last L,
    all real; and an X after CLAW's W that makes no word yet (plain ink)."""
    words = [
        ("SHELL", 7, 5, 0, 1), ("SAND", 7, 5, 1, 0), ("CLAW", 6, 9, 1, 0),
    ]
    placed = {}
    for w, r, c, dr, dc in words:
        for k, l in enumerate(w):
            placed[(r + dr * k, c + dc * k)] = (l, "real")
    placed[(9, 10)] = ("X", "plain")
    rng = random.Random(3)
    out = []
    for (r, c), (l, ink) in sorted(placed.items()):
        t = {"model": "tile", "at": square(r, c), "turn": [0, rng.choice([-2, -1, 0, 1, 2]), 0], "letter": l}
        if ink == "real":
            t["ink"] = "real"
        out.append(t)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args()
    things, piles, loose, tiles, stats = layout(a.seed)
    print(json.dumps(stats))
    tile_things = [t for t in things if t["model"] == "tile"]
    base_note = (
        f"Written by tools/piles.py (seed {a.seed}); regenerate rather than edit. "
        "Square (row r, column c), both 0 to 14, is at x = (c - 7) * 210, z = (r - 7) * 210; the board's top is y = 40. "
    )
    write_scene("seafloor", scene(
        "Seafloor",
        "The seafloor map from high 3/4: the board in the middle, nine sand mounds round it, the 294 tiles "
        "(three standard sets without blanks) in twelve messy piles of 17 to 22 and fifty alone, about a third face down, "
        f"and a few rocks, shells, starfish and weeds. The four crabs start just off the board's corners, at (+-{START}, +-{START}), "
        "kept clear. " + base_note,
        {"target": [0, 0, 0], "yaw": 25, "pitch": 48, "distance": 12500, "fov": 40},
        things, fog=[9000, 26000]))
    # close-ups of two piles from a crab's eye height
    for sid, k, yaw in (("pile-a", 0, 250), ("pile-b", 7, 200)):
        px, pz, _ = PILES[k]
        mine = [tile_things[i] for i in piles[k]]
        cy = max(t["at"][1] for t in mine) // 2
        d = math.radians(yaw)
        eye_r = 950
        ex, ez = px + eye_r * math.sin(d), pz + eye_r * math.cos(d)
        ground = Ground()
        eye_y = round(ground.base(ex, ez)) + 300
        write_scene(sid, scene(
            f"Pile {sid[-1].upper()}",
            f"A messy pile of {len(mine)} tiles close up, from a crab's eye height (300 mm over the sand): tiles "
            "tumbled at random headings, leaning on each other, some face down. " + base_note,
            {"eye": [round(ex), eye_y, round(ez)], "target": [px, cy, pz], "fov": 45},
            near(things, px, pz, 3500), fog=[2500, 9000]))
    # a loose tile alone on the sand: the one nearest the board on flat sand
    prop_at = [t["at"] for t in things if t["model"] not in ("tile", "seafloor", "board") and not t["model"].startswith("mound")]

    def lonely(t):
        x, _, z = t["at"]
        if t.get("face") == "down" or abs(t["turn"][0]) + abs(t["turn"][2]) > 4:
            return 1e9
        if math.hypot(x, z) > 3800 or min(math.hypot(x - q[0], z - q[2]) for q in prop_at) < 1000:
            return 1e9
        return -min(math.hypot(x - tiles[i][0][0], z - tiles[i][0][2]) for i in range(len(tiles)) if tiles[i][0] != t["at"])

    # a face-up one lying flat, far from props and from other tiles
    lone = min((tile_things[i] for i in loose), key=lonely)
    lx, ly, lz = lone["at"]
    d = math.atan2(lx, lz)
    ex, ez = lx + 900 * math.sin(d + 0.4), lz + 900 * math.cos(d + 0.4)
    write_scene("loose-tile", scene(
        "Loose tile",
        "One tile alone on the sand, between the board and the mounds. " + base_note,
        {"eye": [round(ex), ly + 320, round(ez)], "target": [lx, ly + 60, lz], "fov": 40},
        near(things, lx, lz, 3000), fog=[2000, 8000]))
    # behind a crab starting at the board's near-left corner (-x, +z)
    cx, cz = -START, START
    back = 352 / math.sqrt(2)
    write_scene("behind-crab", scene(
        "Behind a crab",
        f"Where the third-person camera stands for a crab starting off the board's near-left corner (-{START}, 0, {START}): "
        "352 mm behind it and 196 mm up (the crab has a 90 mm shell, R1.8), looking toward the board's centre. No crab is in it yet: the crab is added "
        "later; this frame goes under the HUD mock-ups. " + base_note,
        {"eye": [round(cx - back), 196, round(cz + back)], "target": [0, 40, 0], "fov": 55},
        things, fog=[6000, 20000]))
    write_scene("board-above", scene(
        "Board from above",
        "The board straight from above, filling the frame: every premium square's colour and label, and the centre star. "
        + base_note,
        {"target": [0, 40, 0], "yaw": 0, "pitch": 89, "distance": 4750, "fov": 40},
        [{"model": "seafloor"}, {"model": "board"}], fog=[8000, 30000]))
    write_scene("board-tiles", scene(
        "Board with tiles",
        "The board at a low 3/4 angle with a few placed tiles: SHELL across row 7 through the centre, SAND down "
        "column 5 from its S and CLAW down column 9 through its last L, in real-word ink; an X after CLAW's W in plain ink "
        "(no word yet). Tiles stand on y = 40. " + base_note,
        {"target": [0, 40, 100], "yaw": 35, "pitch": 26, "distance": 4300, "fov": 40},
        [{"model": "seafloor"}, {"model": "board"}] + board_tiles(), fog=[4000, 16000]))


if __name__ == "__main__":
    main()
