#!/usr/bin/env python3
"""Writes the title card scenes (data/scenes/title*.json).

    python3 tools/models/title_scenes.py .
"""
import json, math, pathlib, sys

SC = pathlib.Path(sys.argv[1]) / "data/scenes"
WORD = "CRABBLE"
CRAB = ("CRAB SPOT: the crab model is being made by another agent; when it lands, add "
        "{\"model\": \"crab\", ...} peeking over the tile named here.")


def write(id, name, note, camera, things, fog=(2600, 8000)):
    s = {"_note": note, "name": name, "camera": camera, "fog": list(fog), "things": things}
    (SC / f"{id}.json").write_text(json.dumps(s, indent=2) + "\n")


def bubbles(spots):
    return [{"model": "title-bubble", "at": list(a), "scale": s} for a, s in spots]


# A: flat on the sand in a gentle arch, each turned a little, the second B tipped up on a shell
xs = [-720 + 240 * k for k in range(7)]
yaw = [-9, 6, -4, 8, -14, 5, -8]
things = [{"model": "sand-patch"}]
for k, (x, l) in enumerate(zip(xs, WORD)):
    z = round(-110 * (1 - (x / 760) ** 2)) + (18 if k % 2 else -12)
    t = {"model": "tile", "at": [x, 0, z], "turn": [0, yaw[k], 0], "letter": l}
    if k == 4:
        # tipped up toward the camera on a scallop hidden behind it
        t["at"] = [x, 30, z]
        t["turn"] = [16, yaw[k], 7]
    things.append(t)
things += [
    {"model": "title-scallop", "at": [xs[4] + 10, 0, -150], "turn": [0, 170, 0], "scale": 90},
    {"model": "title-scallop", "at": [-900, 0, 260], "turn": [0, 35, 0]},
    {"model": "title-scallop", "at": [960, 0, 120], "turn": [0, -50, 0], "scale": 75},
]
things += bubbles([((-560, 260, -120), 70), ((-610, 430, -160), 45), ((-530, 560, -140), 30),
                   ((640, 330, -200), 90), ((700, 560, -230), 55), ((610, 740, -210), 35), ((660, 880, -260), 22),
                   ((120, 620, -320), 40), ((150, 790, -340), 25)])
write("title-arch", "Title: arch",
      "The title card, take A: CRABBLE in wooden letter tiles lying on the sand in a gentle arch, each turned a little, "
      "the second B tipped up on a scallop shell hidden behind it; two more shells, and bubbles rising at both ends. The "
      "camera keeps the word in the top half and leaves the bottom for the menu's buttons. "
      + CRAB + " Spot: behind the A (x -240), eyes over its top edge, about [-240, 0, -330], facing +z.",
      {"target": [0, 0, 300], "yaw": 0, "pitch": 38, "distance": 1750, "fov": 40}, things)

# B: stood on edge facing the camera, staggered, sunk a little in the sand, one leaning
things = [{"model": "sand-patch"}]
lean = [-6, 4, -3, 2, 14, -4, 7]
sink = [12, 0, 22, 6, 0, 16, 4]
for k, (x, l) in enumerate(zip(xs, WORD)):
    z = (40 if k % 2 else -30) - round(60 * (1 - (x / 760) ** 2))
    t = {"model": "tile", "at": [x, 100 - sink[k], z], "turn": [62 - (10 if k == 4 else 0), yaw[k] // 2, lean[k]], "letter": l}
    things.append(t)
things += [
    {"model": "title-scallop", "at": [-860, 0, 240], "turn": [0, 30, 0]},
    {"model": "title-scallop", "at": [300, 0, 230], "turn": [0, -20, 0], "scale": 70},
    {"model": "title-scallop", "at": [940, 0, 80], "turn": [0, -60, 0], "scale": 85},
]
things += bubbles([((-560, 330, -40), 70), ((-600, 520, -60), 45), ((-520, 660, -50), 28),
                   ((620, 380, -60), 85), ((690, 600, -80), 50), ((600, 780, -90), 32), ((650, 920, -100), 20),
                   ((60, 440, -160), 38), ((100, 620, -170), 24)])
write("title", "Title",
      "The title card, take B: CRABBLE in wooden letter tiles stood on edge facing the camera like a sign, staggered "
      "back and forth, each sunk a little in the sand and leaning its own way, the second B leaning back furthest; "
      "shells at its feet and bubbles rising at both ends. The camera is low and leaves the bottom for the menu. "
      + CRAB + " Spot: behind the first B (x 0), its eyes over the tile's top edge, about [0, 0, -260].",
      {"target": [0, 30, 380], "yaw": 0, "pitch": 15, "distance": 1750, "fov": 40}, things)

# C: tumbled: flat tiles fanning out, the two Bs propped against each other like a tent
things = [{"model": "sand-patch"}]
flat = {0: (-760, -40, -14), 1: (-520, -70, 9), 2: (-280, -95, -5), 5: (420, -80, 12), 6: (660, -40, -10)}
for k, l in enumerate(WORD):
    if k in flat:
        x, z, y = flat[k]
        things.append({"model": "tile", "at": [x, 0, z], "turn": [0, y, 0], "letter": l})
things += [
    # the two Bs lean on each other, their tops touching
    # stood up facing the camera, each rolled 24 degrees in toward the other
    {"model": "tile", "at": [-30, 92, -60], "turn": [80, 0, 24], "letter": "B"},
    {"model": "tile", "at": [170, 92, -60], "turn": [80, 0, -24], "letter": "B"},
    {"model": "title-scallop", "at": [70, 0, 170], "turn": [0, 10, 0], "scale": 85},
    {"model": "title-scallop", "at": [-1000, 0, 200], "turn": [0, 40, 0]},
    {"model": "title-scallop", "at": [930, 0, 230], "turn": [0, -40, 0], "scale": 70},
]
things += bubbles([((-600, 300, -220), 65), ((-640, 480, -250), 40), ((-560, 620, -240), 26),
                   ((70, 420, -260), 80), ((120, 620, -290), 48), ((40, 800, -300), 30), ((90, 950, -320), 18),
                   ((700, 330, -200), 50), ((660, 520, -230), 30)])
write("title-tumbled", "Title: tumbled",
      "The title card, take C: CRABBLE tumbled on the sand: C, R, A, L and E flat and fanned out, the two Bs propped "
      "against each other like a little tent in the middle, a shell at their feet and bubbles rising from behind them. "
      "The camera leaves the bottom for the menu. "
      + CRAB + " Spot: behind the B tent (x 70), peeking over its ridge, about [70, 0, -330].",
      {"target": [0, 0, 300], "yaw": 0, "pitch": 38, "distance": 1750, "fov": 40}, things)
print("ok")
