#!/usr/bin/env python3
"""Writes the title card's small models: title-bubble and title-scallop.

    python3 tools/models/title_models.py .
"""
import json, math, pathlib, sys

MO = pathlib.Path(sys.argv[1]) / "data/models"


def bubble():
    # a ball of 12 sides by 7 rings, radius 50, standing on y = 0; a small
    # bright fleck up and to one side for the shine
    R, SEG, RINGS = 50, 12, 7
    verts = [[0, 0, 0]]
    for i in range(1, RINGS):
        phi = math.pi * i / RINGS
        y = R - R * math.cos(phi)
        r = R * math.sin(phi)
        for k in range(SEG):
            a = 2 * math.pi * (k + (0.5 if i % 2 else 0)) / SEG
            verts.append([round(r * math.cos(a)), round(y), round(r * math.sin(a))])
    top = len(verts)
    verts.append([0, 2 * R, 0])
    faces = []
    ring = lambda i, k: 1 + (i - 1) * SEG + k % SEG
    for k in range(SEG):
        faces.append({"colour": "skin", "v": [0, ring(1, k + 1), ring(1, k)]})
        faces.append({"colour": "skin", "v": [top, ring(RINGS - 1, k), ring(RINGS - 1, k + 1)]})
    for i in range(1, RINGS - 1):
        for k in range(SEG):
            if i % 2:
                faces.append({"colour": "skin", "v": [ring(i, k), ring(i, k + 1), ring(i + 1, k + 1)]})
                faces.append({"colour": "skin", "v": [ring(i, k), ring(i + 1, k + 1), ring(i + 1, k)]})
            else:
                faces.append({"colour": "skin", "v": [ring(i, k), ring(i, k + 1), ring(i + 1, k)]})
                faces.append({"colour": "skin", "v": [ring(i, k + 1), ring(i + 1, k + 1), ring(i + 1, k)]})
    # the shine: a small flat lozenge just outside the ball, up and toward the front left
    n = len(verts)
    cx, cy, cz = 18, 78, 30
    verts += [[cx, cy + 9, cz - 2], [cx + 8, cy, cz + 3], [cx, cy - 9, cz + 6], [cx - 8, cy, cz + 1]]
    faces.append({"colour": "shine", "v": [n, n + 1, n + 2, n + 3]})
    return {
        "_note": "A rising bubble for the title card, 100 mm across (scale it per bubble): a ball of 12 sides by 7 rings, see-through and glowing pale aqua so it reads as air in water, with a small solid fleck of shine up and to the front. It stands on y = 0 (its middle is 50 up). Written by tools/models/title_models.py.",
        "name": "Bubble",
        "kind": "title",
        "unit": "mm",
        "view": {"yaw": 20, "pitch": 15},
        "colours": {"skin": {"rgb": [214, 250, 248], "alpha": 22, "glow": True},
                    "shine": {"rgb": [255, 255, 255], "glow": True}},
        "parts": [{"name": "ball", "pivot": [0, 50, 0], "vertices": verts}],
        "faces": faces,
    }


def scallop():
    # A scallop shell lying on the sand, hinge at the back (-z), fan opening
    # to the front (+z): about 170 mm across and 150 deep (cartoon big: a real
    # one is 50-150 mm). Radiating ribs (ridges and grooves alternate), a low
    # dome, and the two flat "ears" (auricles) either side of the hinge.
    RIBS = 16           # grooves and ridges round the fan
    SPAN = 124          # degrees the fan opens
    RINGS = [0, 35, 75, 115, 150]   # distance out from the hinge
    hinge = [0, 0, -70]
    verts, faces = [], []
    idx = {}
    for j, r in enumerate(RINGS):
        for k in range(RIBS + 1):
            a = math.radians(-SPAN / 2 + SPAN * k / RIBS)
            ridge = (k % 2 == 0)
            rr = r + (6 if ridge and j == len(RINGS) - 1 else 0)
            x = rr * math.sin(a)
            z = hinge[2] + rr * math.cos(a)
            # dome: highest about a third out, sinking to the rim
            t = r / RINGS[-1]
            y = 4 + 46 * math.sin(math.pi * min(1, t * 1.15)) * (1 - 0.35 * t) * (0.6 + 0.4 * math.cos(a))
            if j == len(RINGS) - 1:
                y = 6
            if ridge and 0 < j:
                y += 7 * t
            if j == 0:
                x, y, z = 0, 12, hinge[2]
            idx[(j, k)] = len(verts)
            verts.append([round(x), round(y), round(z)])
    for j in range(len(RINGS) - 1):
        for k in range(RIBS):
            shade = "rib" if k % 2 == 0 else "groove"
            a, b, c, d = idx[(j, k)], idx[(j, k + 1)], idx[(j + 1, k + 1)], idx[(j + 1, k)]
            if j == 0:
                faces.append({"colour": shade, "v": [a, d, c]})
            else:
                faces.append({"colour": shade, "v": [a, d, c]})
                faces.append({"colour": shade, "v": [a, c, b]})
    # the rim down to the sand
    last = len(RINGS) - 1
    base = {}
    for k in range(RIBS + 1):
        x, y, z = verts[idx[(last, k)]]
        base[k] = len(verts)
        verts.append([x, 0, z])
    for k in range(RIBS):
        faces.append({"colour": "lip", "v": [idx[(last, k)], idx[(last, k + 1)], base[k + 1], base[k]]})
    # the two sides of the fan down to the hinge
    for k in (0, RIBS):
        h = len(verts)
        verts.append([0, 0, hinge[2]])
        faces.append({"colour": "lip", "v": [idx[(0, k)], idx[(1, k)], idx[(2, k)], idx[(3, k)], idx[(last, k)], base[k], h]})
    # the ears: flat wedges either side of the hinge
    for s in (-1, 1):
        e = len(verts)
        verts += [[0, 12, hinge[2] - 4], [s * 52, 6, hinge[2] - 6], [s * 46, 6, hinge[2] + 22], [s * 12, 14, hinge[2] + 22],
                  [s * 52, 0, hinge[2] - 6], [s * 46, 0, hinge[2] + 22]]
        faces.append({"colour": "ear", "v": [e, e + 1, e + 2, e + 3]})
        faces.append({"colour": "lip", "v": [e + 1, e + 2, e + 5, e + 4]})
        faces.append({"colour": "lip", "v": [e, e + 1, e + 4]})
    return {
        "_note": "A scallop shell lying on the sand for the title card, cartoon big (about 170 mm across, 150 front to back; a real one is 50-150 mm): a low fan-shaped dome with radiating ribs (ridges and grooves in two coral shades) opening to the front (+z), its hinge at the back with the two flat ears (auricles) either side, as on a real scallop. Written by tools/models/title_models.py.",
        "name": "Scallop shell",
        "kind": "title",
        "unit": "mm",
        "view": {"yaw": 25, "pitch": 40},
        "colours": {"rib": [246, 160, 132], "groove": [226, 128, 108], "lip": [204, 112, 96], "ear": [238, 150, 124]},
        "parts": [{"name": "shell", "pivot": [0, 0, 0], "vertices": verts}],
        "faces": faces,
    }


(MO / "title-bubble.json").write_text(json.dumps(bubble(), indent=1) + "\n")
(MO / "title-scallop.json").write_text(json.dumps(scallop(), indent=1) + "\n")
print("ok")
