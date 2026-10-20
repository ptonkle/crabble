#!/usr/bin/env python3
"""Writes the two starter models that prove the pipeline: data/models/tile.json
(a wooden letter tile) and data/models/sand-patch.json (a patch of sand).

The JSON files are the source of truth the game reads (format: the top of
crates/core/src/model.rs). This script is how their first versions were
drawn; edit a file by hand or change this and run it again:

    python3 tools/models/starters.py

Units are millimetres; +y up, +z the model's front, +x its left. A tile is
200 x 200 x 45 mm (SECOND-ORDER R1.8): a slab with rounded corners, its top
and bottom edges chamfered, the letter's and the value's labels on top.
"""
import json
import math
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]


def rounded_square(half, radius, y, steps=3):
    """A square of half-width `half` with rounded corners, as points at
    height y, going round anticlockwise seen from above (+y)."""
    pts = []
    corners = [(1, 1, 0), (-1, 1, 90), (-1, -1, 180), (1, -1, 270)]
    for sx, sz, a0 in corners:
        cx, cz = sx * (half - radius), sz * (half - radius)
        for k in range(steps + 1):
            a = math.radians(a0 + 90 * k / steps)
            pts.append([round(cx + radius * math.cos(a)), y, round(cz + radius * math.sin(a))])
    return pts


def tile():
    half = 100
    # (half-width, corner radius, height): bottom chamfer, side, top chamfer
    rings = [(96, 10, 0), (100, 14, 5), (100, 14, 37), (92, 8, 45)]
    verts = []
    for h, r, y in rings:
        verts += rounded_square(h, r, y)
    n = len(verts) // len(rings)
    faces = []
    # the top (looking down it winds anticlockwise: its normal is +y), cut
    # across x into strips of two shades: faint grain running up the letter
    top = list(range(3 * n, 4 * n))
    ring = [verts[i] for i in top]
    cuts = [-92, -66, -49, -18, 6, 27, 58, 74, 92]
    for k in range(len(cuts) - 1):
        x0, x1 = cuts[k], cuts[k + 1]
        strip = []
        for i in range(n):
            a, b = ring[i], ring[(i + 1) % n]
            if x0 <= a[0] <= x1:
                strip.append(top[i])
            for x in (x0, x1):
                if (a[0] - x) * (b[0] - x) < 0:
                    # the cut meets this edge (a straight side, z = +-92)
                    t = (x - a[0]) / (b[0] - a[0])
                    q = [x, 45, round(a[2] + t * (b[2] - a[2]))]
                    if q not in verts:
                        verts.append(q)
                    strip.append(verts.index(q))
        # order round the strip's middle, then wind it so its normal is +y
        mx = (x0 + x1) / 2
        strip.sort(key=lambda i: math.atan2(verts[i][2], verts[i][0] - mx))
        faces.append({"colour": "face" if k % 2 == 0 else "grain", "v": strip[::-1]})
    bottom = list(range(0, n))
    faces.append({"colour": "edge", "v": bottom})
    kinds = ["chamfer", "edge", "chamfer"]
    for ring in range(3):
        for k in range(n):
            a, b = ring * n + k, ring * n + (k + 1) % n
            faces.append({"colour": kinds[ring], "v": [a, b, b + n, a + n]})
    return {
        "_note": "A wooden letter tile, 200 x 200 x 45 mm on the board's 210 mm squares (SECOND-ORDER R1.8): a slab with rounded corners, its top and bottom edges chamfered, lighter on its face than on its sides, the face in strips of two close shades for a faint grain running up the letter, like a real wooden word tile (about 4.4 times as wide as it is thick). The origin is the middle of its underside. Its labels: the letter in the middle of the face, a little up and left, and the point value in the bottom right corner as read (the letter's up is -z). Written by tools/models/starters.py.",
        "name": "Tile",
        "kind": "tile",
        "unit": "mm",
        "view": {"yaw": 20, "pitch": 40},
        "colours": {"face": [234, 198, 140], "grain": [226, 188, 128], "chamfer": [222, 182, 124], "edge": [196, 152, 98]},
        "parts": [{"name": "slab", "pivot": [0, 0, 0], "vertices": verts}],
        "faces": faces,
        "anchors": {
            "letter": {"part": "slab", "at": [-12, 45, -12], "right": [1, 0, 0], "up": [0, 0, -1], "size": [110, 110]},
            "value": {"part": "slab", "at": [64, 45, 72], "right": [1, 0, 0], "up": [0, 0, -1], "size": [42, 30], "align": "right"},
        },
    }


def sand_patch():
    # a low-poly patch: a middle, two rings of 16 rising and falling a
    # little (each turned half a step from the last, so the facets are
    # triangles), and a rim sunk under the floor so its edge never shows
    n = 16
    rings = [
        (900, 0.0, [8, 14, 4, 10, 16, 6, 12, 3, 9, 15, 5, 11, 7, 13, 2, 10]),
        (2200, 0.5, [3, 19, 28, 7, 22, 4, 25, 8, 2, 23, 6, 27, 10, 5, 24, 8]),
        (4000, 0.0, [-40] * n),
    ]
    verts = [[0, 6, 0]]
    for radius, turn, heights in rings:
        for k in range(n):
            a = 2 * math.pi * (k + turn) / n
            wobble = 1 + 0.05 * math.sin(3 * a + radius)
            verts.append([round(radius * wobble * math.cos(a)), heights[k], round(radius * wobble * math.sin(a))])
    ring = lambda r, k: 1 + r * n + k % n
    shades = ["sand", "sand_light", "sand"]
    faces = []
    for k in range(n):
        faces.append({"colour": shades[k % 3], "v": [0, ring(0, k + 1), ring(0, k)]})
    for r in range(2):
        for k in range(n):
            # ring r+1 is turned half a step ahead of ring r when r is 0,
            # and half a step behind when r is 1
            a0, a1 = ring(r, k), ring(r, k + 1)
            if r == 0:
                b0, bm = ring(1, k - 1), ring(1, k)
                faces.append({"colour": shades[(k + r) % 3], "v": [a0, a1, bm]})
                faces.append({"colour": shades[(k + r + 1) % 3], "v": [a0, bm, b0]})
            else:
                b0, b1 = ring(2, k), ring(2, k + 1)
                colour = "sand_dark"
                faces.append({"colour": colour, "v": [a0, b1, b0]})
                faces.append({"colour": colour, "v": [a0, a1, b1]})
    return {
        "_note": "A patch of seafloor sand 8 m across, for the Gallery and for scenes: a low, gently uneven disc of flat facets in three sand shades, its rim sunk 40 mm under the floor so its edge never shows. The origin is its middle, on the floor. Written by tools/models/starters.py.",
        "name": "Sand patch",
        "kind": "seafloor",
        "unit": "mm",
        "view": {"yaw": 20, "pitch": 30},
        "colours": {"sand": [232, 206, 166], "sand_light": [240, 218, 180], "sand_dark": [218, 190, 150]},
        "parts": [{"name": "sand", "pivot": [0, 0, 0], "vertices": verts}],
        "faces": faces,
    }


def write(name, model):
    path = ROOT / "data" / "models" / f"{name}.json"
    lines = ["{"]
    keys = list(model)
    for i, k in enumerate(keys):
        comma = "," if i + 1 < len(keys) else ""
        v = model[k]
        if k == "parts":
            parts = []
            for p in v:
                vs = ",\n        ".join(json.dumps(x) for x in p["vertices"])
                parts.append(f'    {{"name": {json.dumps(p["name"])}, "pivot": {json.dumps(p["pivot"])}, "vertices": [\n        {vs}\n    ]}}')
            lines.append(f'  "parts": [\n' + ",\n".join(parts) + f"\n  ]{comma}")
        elif k == "faces":
            fs = ",\n    ".join(json.dumps(f) for f in v)
            lines.append(f'  "faces": [\n    {fs}\n  ]{comma}')
        elif isinstance(v, dict) and k in ("anchors", "colours"):
            inner = ",\n    ".join(f"{json.dumps(a)}: {json.dumps(b)}" for a, b in v.items())
            lines.append(f'  {json.dumps(k)}: {{\n    {inner}\n  }}{comma}')
        else:
            lines.append(f"  {json.dumps(k)}: {json.dumps(v)}{comma}")
    lines.append("}")
    path.write_text("\n".join(lines) + "\n")
    print(path.relative_to(ROOT), len(model["faces"]), "faces")


if __name__ == "__main__":
    write("tile", tile())
    write("sand-patch", sand_patch())
