#!/usr/bin/env python3
"""Writes the word-marks and ghost scenes (data/scenes/words-*.json, ghost-*.json), the board stand-in and the ghost tile.

    python3 tools/models/marks_scenes.py .
"""
import json, pathlib, sys

ROOT = pathlib.Path(sys.argv[1])
SC = ROOT / "data/scenes"
MO = ROOT / "data/models"

SQ = 210
def at(r, c, y=40):
    return [(c - 7) * SQ, y, (r - 7) * SQ]

# (row, col, letter)
TILES = [(7,7,"A"),(7,6,"R"),(7,5,"C"),(7,8,"B"),(6,8,"R"),(5,8,"O"),
         (8,6,"E"),(9,6,"E"),(10,6,"F"),(7,9,"S"),(10,7,"U"),(10,8,"J"),(4,8,"S")]
WORDS = [
    {"from": [7,5], "to": [7,9], "owner": "yellow", "real": True},
    {"from": [4,8], "to": [7,8], "owner": "blue", "real": True},
    {"from": [7,6], "to": [10,6], "owner": "purple", "real": True},
    {"from": [10,6], "to": [10,8], "owner": "red", "real": False},
]
def cells(w):
    (r0,c0),(r1,c1) = w["from"], w["to"]
    if r0 == r1: return [(r0,c) for c in range(c0,c1+1)]
    return [(r,c0) for r in range(r0,r1+1)]
# green only where every word through the letter is real (CR-Q3, R1.180; the core's marks::real_cells)
FAKE = {x for w in WORDS if not w["real"] for x in cells(w)}
REAL = {x for w in WORDS if w["real"] for x in cells(w)} - FAKE

NOTE_BOARD = ("Board squares: square (row r, column c), both 0-14, has its middle at x = (c - 7) * 210, "
              "z = (r - 7) * 210 mm (the centre square is the origin); the board's top is y = 40, where tiles stand. "
              "The board is a flat stand-in (board-stand-in) until the real board model lands.")

def tiles(ghost=None):
    out = [{"model": "sand-patch"}, {"model": "board-stand-in"}]
    for r, c, l in TILES:
        t = {"model": "tile", "at": at(r, c), "letter": l}
        if (r, c) in REAL: t["ink"] = "real"
        out.append(t)
    if ghost:
        out.append(ghost)
    return out

ABOVE = {"target": [0, 0, 0], "yaw": 0, "pitch": 89, "distance": 2700, "fov": 40}
THIRD = {"eye": [150, 760, 1800], "target": [0, 40, 60], "fov": 40}
CLOSE = {"target": [-210, 85, 0], "yaw": -25, "pitch": 48, "distance": 900, "fov": 40}

def scene(id, name, note, camera, things, marks, fog=(3500, 10000)):
    s = {"_note": note + " " + NOTE_BOARD, "name": name, "camera": camera, "fog": list(fog), "things": things}
    if marks: s["marks"] = marks
    (SC / f"{id}.json").write_text(json.dumps(s, indent=2) + "\n")

WORDS_NOTE = ("The example board (the same as the overhead mock-up): CRABS owned by yellow, SORB by blue, REEF by purple "
              "(all real, green letters) and FUJ by red (not real: its outline is dashed with gaps, its letters plain, "
              "F too although it is also in REEF: a letter is green only when every word through it is real, CR-Q3). Shared tiles R (yellow and purple), B (yellow and blue) and F (purple and red) "
              "get dashed borders of alternating colours.")

for lie in ("tiles", "board"):
    m = {"on": lie, "words": WORDS}
    where = "on the tiles' tops" if lie == "tiles" else "on the board round the tiles' feet"
    scene(f"words-above-{lie}", f"Words from above ({lie})", f"{WORDS_NOTE} Outlines lie {where}; seen from straight above.",
          ABOVE, tiles(), m)
    scene(f"words-third-{lie}", f"Words third person ({lie})", f"{WORDS_NOTE} Outlines lie {where}; seen from a crab's camera behind and above.",
          THIRD, tiles(), m)
    scene(f"words-shared-{lie}", f"Shared tile close ({lie})", f"{WORDS_NOTE} Outlines lie {where}; close on the shared R of CRABS and REEF.",
          CLOSE, tiles(), m, fog=(3000, 9000))

GHOST_NOTE = ("A held S's ghost on square (11, 6), under REEF's F: it would make REEFS (a real word, column 6 rows 7-11); "
              "row 11 has nothing beside it, so it forms no other word. ")
gs = {"model": "tile", "at": at(11, 6), "letter": "S"}
words_tiles = {"on": "tiles", "words": WORDS}
scene("ghost-alpha", "Ghost tile: see-through", GHOST_NOTE + "The plain look: the tile at alpha 40.", THIRD,
      tiles(dict(gs, alpha=40)), words_tiles)
scene("ghost-ring", "Ghost tile: ring", GHOST_NOTE + "Alpha 40 with a glowing aqua ring round its foot.", THIRD,
      tiles(dict(gs, alpha=40)), dict(words_tiles, ghost=[11, 6]))
scene("ghost-bob", "Ghost tile: ring and bob", GHOST_NOTE + "Alpha 45, the ring, and the tile at the top of a gentle bob, 30 mm up (the game would bob it about 0 to 30 mm, once a second or so).", THIRD,
      tiles(dict(gs, alpha=45, at=at(11, 6, 70))), dict(words_tiles, ghost=[11, 6]))

# the board stand-in: a 15 x 15 slab 40 mm tall, its top in two shades by square so the squares read
N, H = 15, 39
half = N * SQ // 2
verts = [[x, y, z] for y in (0, H) for (x, z) in ((half, half), (-half, half), (-half, -half), (half, -half))]
faces = [{"colour": "side", "v": [0, 1, 2, 3]}]
for k in range(4):
    a, b = k, (k + 1) % 4
    faces.append({"colour": "side", "v": [a, b, b + 4, a + 4]})
faces.append({"colour": "gap", "v": [7, 6, 5, 4]})
for r in range(N):
    for c in range(N):
        x, z = (c - 7) * SQ, (r - 7) * SQ
        h = SQ // 2 - 3
        i = len(verts)
        verts += [[x + h, H + 1, z + h], [x + h, H + 1, z - h], [x - h, H + 1, z - h], [x - h, H + 1, z + h]]
        faces.append({"colour": "square" if (r + c) % 2 else "square2", "v": [i, i + 1, i + 2, i + 3]})
board = {
    "_note": "A plain flat stand-in for the board while the real board model is made: a slab 3150 mm square and 40 mm tall (15 squares of 210 mm), its top in two pale shades by square so the squares read. Square (row r, column c) has its middle at x = (c - 7) * 210, z = (r - 7) * 210. A box because a board is one. Written by tools/models/marks_scenes.py.",
    "name": "Board stand-in",
    "kind": "board",
    "unit": "mm",
    "view": {"yaw": 20, "pitch": 50},
    "colours": {"side": [176, 140, 96], "gap": [190, 156, 110], "square": [214, 186, 140], "square2": [206, 178, 132]},
    "parts": [{"name": "slab", "pivot": [0, 0, 0], "vertices": verts}],
    "faces": faces,
}
(MO / "board-stand-in.json").write_text(json.dumps(board, indent=1) + "\n")

# the ghost tile: the tile's shape in sea-glass aqua, see-through
tile = json.loads((MO / "tile.json").read_text())
tile["_note"] = ("The ghost tile: a held tile's see-through copy on the square a click would place it. The same shape and labels as "
                 "data/models/tile.json (200 x 200 x 45 mm, origin the middle of its underside), in see-through sea-glass aqua so it "
                 "reads as not placed yet against the wooden tiles and the sand; its face glows a little. Copied from tile.json by the "
                 "tools/models/marks_scenes.py: keep its vertices in step with tile.json.")
tile["name"] = "Ghost tile"
tile["colours"] = {"face": {"rgb": [196, 246, 236], "alpha": 55, "glow": True},
                   "chamfer": {"rgb": [140, 226, 214], "alpha": 50},
                   "edge": {"rgb": [96, 196, 190], "alpha": 50}}
(MO / "ghost-tile.json").write_text(json.dumps(tile, indent=1) + "\n")
scene("ghost-glass", "Ghost tile: sea glass", GHOST_NOTE + "The ghost-tile model (see-through sea-glass aqua, its face glowing a little, the letter solid) with the aqua ring round its foot.", THIRD,
      tiles({"model": "ghost-tile", "at": at(11, 6), "letter": "S"}), dict(words_tiles, ghost=[11, 6]))
print("ok")
