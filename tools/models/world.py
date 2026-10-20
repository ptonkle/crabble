#!/usr/bin/env python3
"""Writes the world's model files: the board, the big seafloor, the sand
mounds and the props round them (rocks, two shells, a starfish, a weed).

    python3 tools/models/world.py

The JSON files under data/models/ are the source of truth the game reads
(format: the top of crates/core/src/model.rs); this script is how their
first versions were drawn. tools/piles.py imports the floor and mound
shapes from here, so the tiles it lays rest on exactly the faces drawn.

Units are millimetres; +y up, +z the model's front, +x its left; a model
stands on y = 0 centred on the origin. The board's square (row r, column
c), both 0 to 14, has its centre at x = (c - 7) * 210, z = (r - 7) * 210;
row 0 is the far edge (-z), column 0 the -x edge.
"""
import json
import math
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]

# ---------------------------------------------------------------- helpers


class Mesh:
    """Parts with vertices and faces over all of them, as the format has it."""

    def __init__(self):
        self.parts = []  # [name, parent, pivot, [verts]]
        self.faces = []
        self.count = 0
        self.coords = []

    def part(self, name, parent=None, pivot=(0, 0, 0)):
        self.parts.append([name, parent, list(pivot), []])

    def v(self, p):
        """Adds a vertex to the latest part; returns its number."""
        q = [round(p[0]), round(p[1]), round(p[2])]
        self.parts[-1][3].append(q)
        self.coords.append(q)
        self.count += 1
        return self.count - 1

    def face(self, colour, vs, away=None, up=False, down=False):
        """A face; wound so its normal points away from the point `away`,
        or up, or down, when asked (the light uses the winding)."""
        vs = list(vs)
        if away is not None or up or down:
            n = self.normal(vs)
            if up or down:
                d = n[1] * (1 if up else -1)
            else:
                c = [sum(self.coords[i][j] for i in vs) / len(vs) for j in range(3)]
                d = sum(n[j] * (c[j] - away[j]) for j in range(3))
            if d < 0:
                vs = vs[::-1]
        self.faces.append({"colour": colour, "v": vs})

    def normal(self, vs):
        """Newell's normal, as the core works it out."""
        n = [0.0, 0.0, 0.0]
        for a, b in zip(vs, vs[1:] + vs[:1]):
            p, q = self.coords[a], self.coords[b]
            n[0] += (p[1] - q[1]) * (p[2] + q[2])
            n[1] += (p[2] - q[2]) * (p[0] + q[0])
            n[2] += (p[0] - q[0]) * (p[1] + q[1])
        return n

    def json_parts(self):
        out = []
        for name, parent, pivot, verts in self.parts:
            d = {"name": name}
            if parent:
                d["parent"] = parent
            d["pivot"] = pivot
            d["vertices"] = verts
            out.append(d)
        return out


def hash01(*k):
    """A steady pseudo-random number in [0, 1) from integers."""
    h = 2166136261
    for x in k:
        h = ((h ^ (x & 0xFFFFFFFF)) * 16777619) & 0xFFFFFFFF
    h ^= h >> 13
    h = (h * 1274126177) & 0xFFFFFFFF
    return (h & 0xFFFFFF) / 0x1000000


def model(note, name, kind, colours, mesh, view=None, anchors=None, animations=None):
    d = {"_note": note, "name": name, "kind": kind, "unit": "mm"}
    if view:
        d["view"] = view
    d["colours"] = colours
    d["parts"] = mesh.json_parts()
    d["faces"] = mesh.faces
    if anchors:
        d["anchors"] = anchors
    if animations:
        d["animations"] = animations
    return d


def write(name, m):
    """One vertex list per line, one face per line: diffable, as starters.py."""
    lines = ["{"]
    keys = list(m.keys())
    for i, k in enumerate(keys):
        comma = "," if i < len(keys) - 1 else ""
        v = m[k]
        if k == "parts":
            lines.append('  "parts": [')
            for j, p in enumerate(v):
                head = {kk: vv for kk, vv in p.items() if kk != "vertices"}
                h = json.dumps(head)[:-1]
                vs = ",\n".join("        " + json.dumps(x) for x in p["vertices"])
                pc = "," if j < len(v) - 1 else ""
                lines.append(f'    {h}, "vertices": [\n{vs}\n    ]}}{pc}')
            lines.append("  ]" + comma)
        elif k in ("faces",):
            lines.append(f'  "{k}": [')
            lines.append(",\n".join("    " + json.dumps(x) for x in v))
            lines.append("  ]" + comma)
        elif k == "anchors":
            lines.append('  "anchors": {')
            items = list(v.items())
            lines.append(",\n".join(f"    {json.dumps(a)}: {json.dumps(b)}" for a, b in items))
            lines.append("  }" + comma)
        else:
            lines.append(f"  {json.dumps(k)}: {json.dumps(v)}{comma}")
    lines.append("}")
    (ROOT / "data" / "models" / f"{name}.json").write_text("\n".join(lines) + "\n")


# ---------------------------------------------------------------- board

SQ = 210  # a square, centre to centre
PLATE = 202  # a square's raised plate (8 mm grooves between)
TOP = 40  # where tiles sit
GROOVE = 35  # the grooves' floor
LIP = 74  # the rim's width beyond the grid
RIM_UP = 10  # the rim stands this far over the squares


def premiums():
    """The standard layout, {(row, col): kind}; built from one quarter by
    the board's mirror symmetry and checked by count."""
    quarter = {
        "3W": [(0, 0), (0, 7), (7, 0)],
        "2W": [(1, 1), (2, 2), (3, 3), (4, 4), (7, 7)],
        "3L": [(1, 5), (5, 1), (5, 5)],
        "2L": [(0, 3), (3, 0), (2, 6), (6, 2), (3, 7), (7, 3), (6, 6)],
    }
    out = {}
    for kind, cells in quarter.items():
        for r, c in cells:
            for rr in (r, 14 - r):
                for cc in (c, 14 - c):
                    out[(rr, cc)] = kind
    counts = {k: sum(1 for v in out.values() if v == k) for k in quarter}
    assert counts == {"3W": 8, "2W": 17, "3L": 12, "2L": 24}, counts
    return out


def board():
    prem = premiums()
    colour_of = {None: "plain", "3W": "word3", "2W": "word2", "3L": "letter3", "2L": "letter2"}
    ink_of = {"3W": "ink_light", "3L": "ink_light", "2W": "ink_dark", "2L": "ink_dark"}
    m = Mesh()
    m.part("slab")
    half = 15 * SQ // 2  # 1575
    edge = half + LIP
    # the body: a slab from the sand to the rim's top, its rim bevelled
    rim_top = TOP + RIM_UP
    ring = lambda h, y: [m.v((x, y, z)) for x, z in ((h, h), (-h, h), (-h, -h), (h, -h))]
    base = ring(edge - 6, 0)
    side_lo = ring(edge, 6)
    side_hi = ring(edge, rim_top - 6)
    lip = ring(edge - 8, rim_top)
    inner_top = ring(half + 10, rim_top)
    inner_lo = ring(half + 4, GROOVE)
    m.face("rim_side", base, down=True)
    for a, b, col, out in ((base, side_lo, "rim_side", True), (side_lo, side_hi, "rim_side", True),
                           (side_hi, lip, "rim", True), (lip, inner_top, "rim", False),
                           (inner_top, inner_lo, "rim_inner", False)):
        for k in range(4):
            j = (k + 1) % 4
            if out:
                m.face(col, [a[k], a[j], b[j], b[k]], away=(0, 20, 0))
            else:
                m.face(col, [a[k], a[j], b[j], b[k]], up=True)
    m.face("groove", inner_lo, up=True)
    # the squares: plates standing from the grooves to the top
    anchors = {}
    for r in range(15):
        for c in range(15):
            cx, cz = (c - 7) * SQ, (r - 7) * SQ
            h = PLATE / 2
            kind = prem.get((r, c))
            col = colour_of[kind]
            corners = ((cx + h, cz + h), (cx - h, cz + h), (cx - h, cz - h), (cx + h, cz - h))
            top = [m.v((x, TOP, z)) for x, z in corners]
            low = [m.v((x, GROOVE, z)) for x, z in corners]
            m.face(col, top, up=True)
            for k in range(4):
                j = (k + 1) % 4
                m.face("plate_side", [top[k], top[j], low[j], low[k]], away=(cx, 37, cz))
            if kind and (r, c) != (7, 7):
                anchors[f"r{r}c{c}"] = {
                    "part": "slab", "at": [cx, TOP, cz],
                    "right": [1, 0, 0], "up": [0, 0, -1], "size": [128, 70],
                    "text": kind, "ink": ink_of[kind],
                }
    # the centre star, 1 mm proud: a pentagon and five points
    cx = cz = 0
    outer, inner = 80, 33
    pts = []
    for k in range(10):
        a = math.radians(90 + 36 * k)
        rr = outer if k % 2 == 0 else inner
        pts.append((cx + rr * math.cos(a), cz - rr * math.sin(a)))
    y = TOP + 1
    centre = [m.v((x, y, z)) for x, z in pts[1::2]]
    m.face("star", centre, up=True)
    for k in range(5):
        tip = m.v((pts[2 * k][0], y, pts[2 * k][1]))
        a = centre[(k - 1) % 5]
        b = centre[k]
        m.face("star", [a, tip, b], up=True)
    listing = {}
    for (r, c), k in sorted(prem.items()):
        listing.setdefault(k, []).append(f"{r},{c}")
    note = (
        "The board: 15 x 15 squares of 210 mm on a wooden slab whose top, where tiles sit, is at y = 40; "
        "each square a 202 mm plate between 8 mm grooves 5 mm deep, inside a rim 74 mm wide standing 10 mm over the squares. "
        "Square (row r, column c), both 0 to 14, has its centre at x = (c - 7) * 210, z = (r - 7) * 210 (the centre square is the origin; "
        "row 0 is the far edge, -z). The standard premium layout, as (row,col): "
        + "; ".join(f"{k}: " + " ".join(v) for k, v in sorted(listing.items()))
        + ". The centre (7,7) is a double word marked by a star. Each other premium is stamped with its label (3W, 2W, 3L, 2L), white on the triple squares "
        "and brown on the double ones, so colour is not the only cue. The premium colours keep clear of the four player colours. Written by tools/models/world.py."
    )
    colours = {
        "plain": [235, 214, 166],
        "word3": [217, 115, 78],
        "word2": [240, 179, 194],
        "letter3": [62, 143, 176],
        "letter2": [166, 220, 232],
        "plate_side": [196, 166, 116],
        "groove": [168, 132, 88],
        "rim": [160, 108, 64],
        "rim_inner": [140, 94, 56],
        "rim_side": [124, 82, 50],
        "star": [150, 72, 92],
        "ink_light": [255, 255, 255],
        "ink_dark": [104, 66, 40],
    }
    return model(note, "Board", "board", colours, m, view={"yaw": 0, "pitch": 60}, anchors=anchors)


# ---------------------------------------------------------------- polar surfaces


class Polar:
    """A surface over rings of points round the origin: rings[i] has the
    radius radii[i]; heights[i][k] is ring i's height at sector k (of n).
    Ring 0 may be radius 0 (one point). Triangulated the same way in the
    model file and in `height`, so piles rest on the drawn faces."""

    def __init__(self, radii, n, height_fn):
        self.radii = radii
        self.n = n
        self.h = []
        self.pts = []
        for i, r in enumerate(radii):
            row = []
            prow = []
            for k in range(n if r > 0 else 1):
                a = 2 * math.pi * k / n
                # odd rings turn half a sector, so the triangles interlock
                if i % 2:
                    a += math.pi / n
                x, z = r * math.cos(a), r * math.sin(a)
                y = round(height_fn(x, z, i, k))
                prow.append((round(x), y, round(z)))
            self.pts.append(prow)

    def triangles(self):
        """Each face as three (ring, sector) index pairs."""
        out = []
        n = self.n
        for i in range(len(self.radii) - 1):
            a, b = self.pts[i], self.pts[i + 1]
            if len(a) == 1:
                for k in range(n):
                    out.append(((i, 0), (i + 1, (k + 1) % n), (i + 1, k)))
                continue
            for k in range(n):
                k1 = (k + 1) % n
                if (i + 1) % 2:  # ring i+1 is turned half a sector ahead
                    out.append(((i, k), (i, k1), (i + 1, k)))
                    out.append(((i, k1), (i + 1, k1), (i + 1, k)))
                else:  # ring i is ahead
                    out.append(((i, k), (i + 1, k1), (i + 1, k)))
                    out.append(((i, k), (i, k1), (i + 1, k1)))
        return out

    def mesh(self, m, colour_of):
        idx = {}
        for i, row in enumerate(self.pts):
            for k, p in enumerate(row):
                idx[(i, k)] = m.v(p)
        for t, tri in enumerate(self.triangles()):
            ps = [self.pts[i][k] for i, k in tri]
            vs = [idx[q] for q in tri]
            m.face(colour_of(t, tri, ps), vs, up=True)

    def height(self, x, z):
        """The drawn surface's height at (x, z), or None outside it."""
        r = math.hypot(x, z)
        if r >= self.radii[-1]:
            return None
        # the band of rings, then every triangle in it (few)
        i = max(j for j, rr in enumerate(self.radii) if rr <= r) if r > 0 else 0
        i = min(i, len(self.radii) - 2)
        best = None
        for band in (i, i - 1, i + 1):
            if not 0 <= band < len(self.radii) - 1:
                continue
            for tri in self._band(band):
                h = bary(x, z, [self.pts[a][b] for a, b in tri])
                if h is not None:
                    return h
        return best

    def _band(self, i):
        if not hasattr(self, "_bands"):
            self._bands = {}
            for tri in self.triangles():
                self._bands.setdefault(min(a for a, _ in tri), []).append(tri)
        return self._bands.get(i, [])


def bary(x, z, ps):
    (x0, y0, z0), (x1, y1, z1), (x2, y2, z2) = ps
    d = (z1 - z2) * (x0 - x2) + (x2 - x1) * (z0 - z2)
    if d == 0:
        return None
    a = ((z1 - z2) * (x - x2) + (x2 - x1) * (z - z2)) / d
    b = ((z2 - z0) * (x - x2) + (x0 - x2) * (z - z2)) / d
    c = 1 - a - b
    e = -1e-6
    if a < e or b < e or c < e:
        return None
    return a * y0 + b * y1 + c * y2


# ---------------------------------------------------------------- seafloor

FLOOR_RADII = [0, 1900, 2600, 3300, 4000, 4800, 5600, 6600, 7800, 9200, 11000, 13500, 16000, 19000]
FLOOR_N = 28


def floor_fn(x, z, i, k):
    r = math.hypot(x, z)
    if r < 2000:  # flat under the board and round it
        return 0
    if i == len(FLOOR_RADII) - 1:
        return -60  # the rim, sunk and lost in the fog
    near = min(1, (r - 2000) / 2500)
    amp = 22 * near + max(0, r - 6500) / 7000 * 420
    a = math.atan2(z, x)
    wave = (math.sin(a * 3 + r / 1700) * 0.6 + math.sin(a * 5 - r / 2300 + 1.3) * 0.4
            + (hash01(i, k, 7) - 0.5) * 0.5)
    return amp * wave


FLOOR = Polar(FLOOR_RADII, FLOOR_N, floor_fn)


def seafloor():
    m = Mesh()
    m.part("sand")

    def colour(t, tri, ps):
        h = hash01(t, 3)
        return "sand_light" if h < 0.3 else ("sand_dark" if h > 0.75 else "sand")

    FLOOR.mesh(m, colour)
    note = (
        "The seafloor the board lies on: a low-poly disc of sand 38 m across, flat within 2 m of the middle "
        "(under the board and the crabs' corners), rolling a few centimetres out to the mounds, "
        "then into low dunes that the fog takes; its rim sunk 60 mm under the floor. "
        "Triangles in three sand shades. Written by tools/models/world.py; tools/piles.py lays tiles on these very faces."
    )
    colours = {"sand": [232, 206, 166], "sand_light": [240, 218, 180], "sand_dark": [220, 192, 152]}
    return model(note, "Seafloor", "seafloor", colours, m, view={"yaw": 20, "pitch": 35})


# ---------------------------------------------------------------- mounds

MOUNDS = {
    # id: (name, x radius, z radius, height, wobble seed, note)
    "mound-dome": ("Mound, domed", 1150, 1050, 340, 1, "a domed heap"),
    "mound-long": ("Mound, long", 1650, 900, 300, 2, "a long ridge"),
    "mound-low": ("Mound, low", 1400, 1300, 200, 3, "a wide, low swell"),
}
MOUND_RINGS = 7
MOUND_N = 18


def mound_surface(mid):
    _, rx, rz, height, seed, _ = MOUNDS[mid]
    radii = [0] + [k / MOUND_RINGS for k in range(1, MOUND_RINGS + 1)] + [1.08]

    def fn(x, z, i, k):
        # x, z here are on the unit disc; the shape is stretched afterwards
        rho = math.hypot(x, z)
        if i == len(radii) - 1:
            return -30
        a = math.atan2(z, x)
        wob = 1 + 0.12 * math.sin(2 * a + seed) + 0.07 * math.sin(3 * a + 2 * seed)
        rr = min(1, rho / wob)
        h = height * (0.5 + 0.5 * math.cos(math.pi * rr))
        h += (hash01(seed, i, k) - 0.5) * 18 * (1 - rr)
        return max(h, 0 if i < len(radii) - 2 else -10)

    unit = Polar([r * 1000 for r in radii], MOUND_N, lambda x, z, i, k: fn(x / 1000, z / 1000, i, k))
    # stretch to the mound's radii
    unit.pts = [[(round(x * rx / 1000), y, round(z * rz / 1000)) for x, y, z in row] for row in unit.pts]
    unit.stretch = (rx, rz)
    return unit


def mound_height(mid, x, z):
    """The mound's drawn height at its own (x, z), or None off it."""
    s = MOUND_CACHE.get(mid)
    if s is None:
        s = MOUND_CACHE[mid] = mound_surface(mid)
    rx, rz = s.stretch
    # in the unit disc the radii are as built; check the band there, then
    # use the stretched points, which keep the same triangles
    ux, uz = x * 1000 / rx, z * 1000 / rz
    r = math.hypot(ux, uz)
    if r >= s.radii[-1]:
        return None
    i = max(j for j, rr in enumerate(s.radii) if rr <= r) if r > 0 else 0
    i = min(i, len(s.radii) - 2)
    for band in (i, i - 1, i + 1):
        if not 0 <= band < len(s.radii) - 1:
            continue
        for tri in s._band(band):
            h = bary(x, z, [s.pts[a][b] for a, b in tri])
            if h is not None:
                return h
    return None


MOUND_CACHE = {}


def mound(mid):
    name, rx, rz, height, seed, what = MOUNDS[mid]
    s = mound_surface(mid)
    MOUND_CACHE[mid] = s
    m = Mesh()
    m.part("sand")

    def colour(t, tri, ps):
        h = hash01(t, seed, 11)
        top = sum(p[1] for p in ps) / 3 > height * 0.55
        if top:
            return "crest" if h < 0.6 else "sand"
        return "sand" if h < 0.45 else ("shade" if h < 0.85 else "crest")

    s.mesh(m, colour)
    note = (
        f"A low sand mound, {what}: {2 * rx // 10 / 100:g} by {2 * rz // 10 / 100:g} m and {height} mm high, "
        "a soft cosine dome with a wobbly outline, gentle enough for a crab to walk up (under 25 degrees). "
        "Tiles are found in piles on the mounds. Its rim is sunk 30 mm under the floor. "
        "Written by tools/models/world.py; tools/piles.py lays tiles on these very faces."
    )
    colours = {"sand": [222, 189, 145], "shade": [206, 172, 130], "crest": [236, 208, 166]}
    return model(note, name, "prop", colours, m, view={"yaw": 25, "pitch": 30})


# ---------------------------------------------------------------- rocks


def rock(seed, rx, ry, rz, sides=9, bands=4):
    """A rounded boulder: a squashed lat-long ball, jittered, sunk 15%."""
    m = Mesh()
    m.part("rock")
    rows = []
    for b in range(1, bands):
        lat = math.pi / 2 - math.pi * b / bands
        row = []
        for k in range(sides):
            a = 2 * math.pi * (k + 0.5 * (b % 2)) / sides
            j = 1 + (hash01(seed, b, k) - 0.5) * 0.28
            x = rx * math.cos(lat) * math.cos(a) * j
            z = rz * math.cos(lat) * math.sin(a) * j
            y = ry * (math.sin(lat) * 0.5 + 0.5) * (1 + (hash01(seed, k, b, 5) - 0.5) * 0.2)
            row.append(m.v((x, y - ry * 0.12, z)))
        rows.append(row)
    top = m.v((rx * 0.08, ry * 0.98 - ry * 0.12, -rz * 0.05))
    bottom = m.v((0, -ry * 0.12, 0))
    count = 0

    def tri(vs):
        nonlocal count
        count += 1
        c = hash01(seed, count, 9)
        m.face("rock_light" if c < 0.3 else ("rock_dark" if c > 0.72 else "rock"), vs, away=(0, ry * 0.3, 0))

    for k in range(sides):
        k1 = (k + 1) % sides
        tri([top, rows[0][k1], rows[0][k]])
        tri([bottom, rows[-1][k], rows[-1][k1]])
    for b in range(len(rows) - 1):
        a, c = rows[b], rows[b + 1]
        for k in range(sides):
            k1 = (k + 1) % sides
            if b % 2 == 0:
                tri([a[k], a[k1], c[k]])
                tri([a[k1], c[k1], c[k]])
            else:
                tri([a[k], c[k1], c[k]])
                tri([a[k], a[k1], c[k1]])
    return m


def rocks():
    colours = {"rock": [132, 144, 140], "rock_light": [156, 168, 160], "rock_dark": [104, 116, 116]}
    out = {}
    out["boulder"] = model(
        "A rounded seafloor boulder about 640 mm across and 400 mm high, sunk a little into the sand: a squashed, "
        "jittered ball of flat facets in three grey-greens. Written by tools/models/world.py.",
        "Boulder", "prop", colours, rock(4, 330, 420, 300, sides=11, bands=5), view={"yaw": 30, "pitch": 25})
    out["rock-flat"] = model(
        "A flat seafloor rock about 900 by 620 mm and 240 mm high, a slab worn smooth. "
        "Written by tools/models/world.py.",
        "Rock, flat", "prop", colours, rock(8, 460, 270, 310, sides=10, bands=3), view={"yaw": 30, "pitch": 30})
    out["pebble"] = model(
        "A pebble about 180 mm across, for scattering by the rocks. Written by tools/models/world.py.",
        "Pebble", "prop", colours, rock(13, 95, 110, 80, sides=7, bands=3), view={"yaw": 30, "pitch": 30})
    return out


# ---------------------------------------------------------------- shells


def scallop():
    """A scallop lying cupped side up: a round fan, domed, with low radial
    ribs from the hinge (back, -z) to a scalloped edge, and two ears at
    the hinge."""
    m = Mesh()
    m.part("shell")
    ribs = 11
    spread = math.radians(230)
    radius = 165
    dome = 62
    hinge_z = -radius * 0.62
    cz = 0
    # the outline: a circle round (0, cz) from one ear to the other
    n = 2 * ribs + 1
    rings = []
    for t in (0.4, 0.75, 1.0):
        row = []
        for k in range(n):
            a = -spread / 2 + spread * k / (n - 1)
            rib = k % 2 == 1
            rr = radius * t * (1.04 if rib and t == 1.0 else 1.0)
            # from the hinge outward, a dome: highest a third of the way out
            x = rr * math.sin(a) * (0.55 + 0.45 * t)
            z = hinge_z + (cz - hinge_z + rr * math.cos(a)) * (0.35 + 0.65 * t) if t < 1 else cz + rr * math.cos(a)
            if t < 1:
                x = (radius * math.sin(a)) * t
                z = hinge_z + ((cz + radius * math.cos(a)) - hinge_z) * t
            y = dome * (1 - t ** 2.2) + (9 if rib else 0) * (0.5 + 0.5 * t) + (4 if t == 1.0 else 0)
            row.append(m.v((x, y, z)))
        rings.append(row)
    hinge = m.v((0, dome * 0.7, hinge_z))
    for k in range(n - 1):
        col = "shell" if k % 2 == 0 else "shell_rib"
        m.face(col, [hinge, rings[0][k], rings[0][k + 1]], up=True)
        for i in range(2):
            a, b = rings[i], rings[i + 1]
            m.face(col, [a[k], b[k], b[k + 1]], up=True)
            m.face(col, [a[k], b[k + 1], a[k + 1]], up=True)
    # the underside, flat on the sand
    under = [m.v((radius * math.sin(-spread / 2 + spread * k / 10) * 0.97, 0,
                  cz + radius * math.cos(-spread / 2 + spread * k / 10) * 0.97)) for k in range(11)]
    hinge0 = m.v((0, 0, hinge_z))
    m.face("shell_under", [hinge0] + under, down=True)
    # the ears either side of the hinge
    for s in (1, -1):
        e0 = m.v((s * 6, dome * 0.62, hinge_z - 4))
        e1 = m.v((s * 70, 10, hinge_z - 18))
        e2 = m.v((s * 72, 8, hinge_z + 24))
        e3 = m.v((s * 20, dome * 0.5, hinge_z + 26))
        m.face("shell_ear", [e0, e1, e2, e3], up=True)
    return m


def whelk():
    """A spiral sea shell lying on its side: a big body whorl with an oval
    pink mouth, tapering through bumpy whorls to a pointed spire; a band
    winds round it. Its axis runs along x, the spire toward +x."""
    m = Mesh()
    m.part("shell")
    sides = 10
    # (x along the axis, radius) from the canal to the spire
    profile = [(-170, 16), (-135, 40), (-95, 78), (-50, 98), (0, 96), (35, 80), (52, 66),
               (70, 64), (92, 50), (104, 42), (120, 40), (138, 29), (148, 24), (162, 22), (178, 14)]
    rings = []
    for i, (x, r) in enumerate(profile):
        row = []
        for k in range(sides):
            b = 2 * math.pi * k / sides
            # the band winds: the ring turns a little each step
            row.append(m.v((x, r + r * math.sin(b), r * math.cos(b) * 0.98)))
        rings.append(row)
    for i in range(len(rings) - 1):
        for k in range(sides):
            k1 = (k + 1) % sides
            band = (k - i) % sides in (0, 1)
            col = "whelk_band" if band else ("whelk_light" if (i + k) % 2 else "whelk")
            ax = (profile[i][0] + profile[i + 1][0]) / 2
            ar = (profile[i][1] + profile[i + 1][1]) / 2
            m.face(col, [rings[i][k], rings[i][k1], rings[i + 1][k1]], away=(ax, ar, 0))
            m.face(col, [rings[i][k], rings[i + 1][k1], rings[i + 1][k]], away=(ax, ar, 0))
    canal = m.v((-190, 18, 0))
    tip = m.v((196, 12, 0))
    for k in range(sides):
        k1 = (k + 1) % sides
        m.face("whelk_band", [rings[0][k], rings[0][k1], canal], away=(-150, 16, 0))
        m.face("whelk_band", [rings[-1][k], rings[-1][k1], tip], away=(170, 14, 0))
    # the mouth: an oval on the body whorl's -z side, standing just proud
    mouth = []
    for k in range(10):
        a = 2 * math.pi * k / 10
        x = -60 + 80 * math.cos(a)
        y = 92 + 52 * math.sin(a)
        mouth.append(m.v((x, y, -104 + 10 * (1 - abs(math.cos(a))))))
    m.face("whelk_mouth", mouth, away=(-60, 92, 0))
    inner = [m.v((-60 + 50 * math.cos(2 * math.pi * k / 8), 90 + 30 * math.sin(2 * math.pi * k / 8), -112)) for k in range(8)]
    m.face("whelk_inside", inner, away=(-60, 92, 0))
    return m


def starfish():
    """Five plump tapering arms round a domed middle, each arm a low ridge
    rounding off at the tip, about 420 mm across."""
    m = Mesh()
    m.part("star")
    arms = 5
    centre = m.v((0, 66, 0))
    base = {}
    for k in range(arms):
        a = math.radians(90 + 72 * k + 36)
        base[k] = m.v((80 * math.cos(a), 4, 80 * math.sin(a)))
    for k in range(arms):
        a = math.radians(90 + 72 * k)
        ca, sa = math.cos(a), math.sin(a)
        px, pz = -sa, ca
        at = lambda d, w, y: m.v((ca * d + px * w, y, sa * d + pz * w))
        mid = at(130, 0, 48)
        ml, mr = at(130, 46, 6), at(130, -46, 6)
        end = at(185, 0, 30)
        tl, tr = at(190, 24, 6), at(190, -24, 6)
        tip = at(215, 0, 8)
        lw, rw = base[k], base[(k - 1) % arms]
        m.face("star_top", [centre, mid, lw], up=True)
        m.face("star_top", [centre, rw, mid], up=True)
        m.face("star", [lw, mid, ml], up=True)
        m.face("star", [rw, mr, mid], up=True)
        m.face("star", [ml, mid, end, tl], up=True)
        m.face("star", [mr, tr, end, mid], up=True)
        m.face("star_top", [tl, end, tip], up=True)
        m.face("star_top", [tr, tip, end], up=True)
        m.face("star_under", [lw, ml, tl, tip, tr, mr, rw], down=True)
    m.face("star_under", [base[k] for k in range(arms)], down=True)
    return m


def shells():
    out = {}
    out["scallop"] = model(
        "A scallop shell, about 340 mm across: a domed round fan of eleven low ribs fanning from the hinge to a scalloped edge, cupped side up, "
        "with the two ears at the hinge; coral and cream. Cartoon-big beside a 200 mm tile. Written by tools/models/world.py.",
        "Scallop", "prop",
        {"shell": [244, 168, 128], "shell_rib": [250, 212, 178], "shell_under": [226, 150, 116], "shell_ear": [236, 156, 118]},
        scallop(), view={"yaw": 15, "pitch": 40})
    out["whelk"] = model(
        "A spiral sea shell lying on its side, about 390 mm long: a big body whorl tapering through bumpy whorls to a pointed spire, "
        "cream with a brown band winding round it and a pink mouth. Written by tools/models/world.py.",
        "Spiral shell", "prop",
        {"whelk": [238, 214, 178], "whelk_light": [248, 230, 198], "whelk_band": [176, 112, 76], "whelk_mouth": [244, 170, 172], "whelk_inside": [196, 116, 128]},
        whelk(), view={"yaw": 200, "pitch": 25})
    out["starfish"] = model(
        "A starfish, about 430 mm across: five plump tapering arms, each a low ridge, round a domed middle; orange with "
        "a paler top. Written by tools/models/world.py.",
        "Starfish", "prop",
        {"star": [232, 112, 62], "star_top": [246, 150, 84], "star_under": [204, 96, 56]},
        starfish(), view={"yaw": 10, "pitch": 55})
    return out


# ---------------------------------------------------------------- weed


def weed():
    """A tuft of six ribbon blades, each four segments chained so the sway
    bends them more toward the tip."""
    m = Mesh()
    m.part("root")
    m.v((0, 0, 0))
    m.v((30, 0, 0))
    m.v((0, 0, 30))
    m.face("weed_dark", [0, 1, 2])
    blades = 5
    segs = 4
    keys_a, keys_b, keys_c = {}, {}, {}
    for b in range(blades):
        a = 2 * math.pi * b / blades + hash01(b, 2) * 0.6
        lean = 0.12 + hash01(b, 3) * 0.12
        length = 600 + hash01(b, 4) * 380
        base = (math.cos(a) * 40, 0, math.sin(a) * 40)
        width = 96 + hash01(b, 5) * 30
        # the blade's flat faces across, at right angles to its lean
        wx, wz = -math.sin(a), math.cos(a)
        prev = None
        for s in range(segs):
            name = f"blade{b}_{s}"
            y0 = length * s / segs
            r0 = 40 + lean * y0
            p0 = (math.cos(a) * r0, y0, math.sin(a) * r0)
            m.part(name, parent=prev or "root", pivot=[round(c) for c in p0])
            t = (s + 1) / segs
            y1 = length * t
            r1 = 40 + lean * y1
            w = width * (1 - t * 0.45) / 2
            p1 = (math.cos(a) * r1, y1, math.sin(a) * r1)
            if s == 0:
                w0 = width / 2
                i0 = m.v((p0[0] + wx * w0, p0[1], p0[2] + wz * w0))
                i1 = m.v((p0[0] - wx * w0, p0[1], p0[2] - wz * w0))
                lo = (i0, i1)
            if s < segs - 1:
                j0 = m.v((p1[0] + wx * w, p1[1], p1[2] + wz * w))
                j1 = m.v((p1[0] - wx * w, p1[1], p1[2] - wz * w))
                col = "weed" if (s + b) % 2 else "weed_light"
                m.face(col, [lo[0], lo[1], j1, j0])
                lo = (j0, j1)
            else:
                tip = m.v(p1)
                m.face("weed_light", [lo[0], lo[1], tip])
            prev = name
            # sway: each segment bends a little more, phase by blade
            amp = 4 + 3 * s
            ph = hash01(b, 6) * 2 * math.pi
            for keys, k in ((keys_a, 0), (keys_b, 1), (keys_c, 2)):
                q = ph + 2 * math.pi * k / 3
                keys[name] = {"turn": [round(amp * math.sin(q)), 0, round(amp * math.cos(q))]}
    anim = {"sway": {"ms": 3600, "ease": "smooth", "keys": [
        {"at": 0, "parts": keys_a}, {"at": 1200, "parts": keys_b}, {"at": 2400, "parts": keys_c}]}}
    return model(
        "A tuft of sea weed: five broad ribbon blades 0.6 to 1 m tall leaning out from one root, each in four segments that bend "
        "more toward the tip as it sways in the current (sway, 3.6 s round). Written by tools/models/world.py.",
        "Weed", "prop", {"weed": [78, 146, 92], "weed_light": [108, 172, 104], "weed_dark": [56, 112, 72]},
        m, view={"yaw": 20, "pitch": 15}, animations=anim)


# ---------------------------------------------------------------- all


def main():
    write("board", board())
    write("seafloor", seafloor())
    for mid in MOUNDS:
        write(mid, mound(mid))
    for k, v in rocks().items():
        write(k, v)
    for k, v in shells().items():
        write(k, v)
    write("weed", weed())


if __name__ == "__main__":
    main()
