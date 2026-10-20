#!/usr/bin/env python3
"""Writes data/models/crab.json, the one crab every player steers, and its
animations. The JSON file is the source of truth the game reads (format: the
top of crates/core/src/model.rs); this script is how it is drawn. Change the
numbers here and run it again:

    python3 tools/models/crab.py

Units are millimetres; +y up, +z the crab's front (eyes and claws), +x its
left. The crab stands on y = 0 centred on the origin.

Anatomy (a green or blue shore crab, SECOND-ORDER R1.90):
- Above: the carapace is wider than long (476 mm across the teeth, 361 long, in the design
  sizes; 262 and 199 in the file after SCALE), widest across
  its front third just behind the eyes; the front-side (anterolateral)
  margin carries five teeth from the eye socket to the widest point; behind
  that the sides taper in to a narrower straight back edge.
- Side: the carapace is gently domed, highest a little behind the middle,
  its margin a thin rim; the body below is a flatter plate (the sternum).
- Front: two eyes on short stalks sit in notches at the front middle, short
  antennae between them; the mouthparts sit under the front edge.
- Claws (chelipeds) come from the front corners under the shell: a short
  arm (merus) out and forward, a wrist (carpus) back in, a big palm
  (propodus) ending in a fixed finger below and a movable finger (dactyl)
  hinged on top; held folded in front of the face.
- Four pairs of walking legs come from under the sides, splayed sideways:
  each runs out under the shell margin, rises to a high knee (merus to
  carpus) above the shell, then angles down to a pointed tip (dactyl). They
  bend in the vertical plane across the body (crabs walk sideways); the
  back pair is swept back, the front pair a little forward.
- Below: the abdomen is folded flat under the body against the sternum, a
  broad rounded flap (a female's), 200 mm wide in design sizes (110 in the
  file, about half a tile: the person had the crab shrunk, R1.8).

Part names (for leg IK later): body; eye_l, eye_r; claw_l_arm, claw_l_wrist,
claw_l_palm, claw_l_finger (and claw_r_...); leg_l1_hip, leg_l1_knee,
leg_l1_foot for the left legs 1 (front) to 4 (back), and leg_r1_... for the
right. Each leg part is named after the joint it turns about: _hip carries
the coxa and the long merus up to the knee, _knee the carpus and propodus
down to the ankle, _foot the pointed dactyl. Anchors: claw_left and
claw_right, the middle of a held tile's underside, on the palm parts.
"""
import json
import math
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]

# The shapes below are drawn in design heights; every vertex, pivot and
# anchor comes down by DROP so the leg tips stand on y = 0.
DROP = 25

# The whole crab is drawn at its first size (a 476 mm shell) and then scaled
# by SCALE on the way out: every vertex, pivot, anchor and animation move.
# Turns are angles and do not scale; a held tile keeps its own size. The
# person, 2026-10-19: "crabs are way too big compares to the tiles"
# (SECOND-ORDER R1.8).
SCALE = 0.55

# ---- small vector helpers ----


def add(a, b):
    return [a[0] + b[0], a[1] + b[1], a[2] + b[2]]


def sub(a, b):
    return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]


def mul(a, s):
    return [a[0] * s, a[1] * s, a[2] * s]


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def norm(a):
    n = math.sqrt(dot(a, a))
    return [a[0] / n, a[1] / n, a[2] / n]


def lerp(a, b, t):
    return [a[k] + (b[k] - a[k]) * t for k in range(3)]


# rotation matrices as rows
def matmul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def rot_axis(axis, deg):
    x, y, z = norm(axis)
    c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    t = 1 - c
    return [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
    ]


def turn_of(r):
    """The file's turn [x, y, z] (applied z, then x, then y: R = Ry Rx Rz)
    for a rotation matrix."""
    b = math.degrees(math.asin(max(-1, min(1, -r[1][2]))))
    a = math.degrees(math.atan2(r[0][2], r[2][2]))
    c = math.degrees(math.atan2(r[1][0], r[1][1]))
    return [round(b), round(a), round(c)]


# ---- the mesh being built ----


def out_mm(p):
    """A design point as written to the file: dropped onto the sand, scaled."""
    return [round(p[0] * SCALE), round((p[1] - DROP) * SCALE), round(p[2] * SCALE)]


class Mesh:
    def __init__(self):
        self.parts = []  # {name, parent, pivot, vertices}
        self.faces = []
        self.count = 0

    def part(self, name, parent, pivot):
        p = {"name": name, "pivot": out_mm(pivot), "vertices": []}
        if parent:
            p["parent"] = parent
        self.parts.append(p)
        self.cur = p
        return p

    def v(self, p):
        self.cur["vertices"].append(out_mm(p))
        self.count += 1
        return self.count - 1

    def face(self, colour, vs):
        self.faces.append({"colour": colour, "v": vs})

    def quad(self, colour, a, b, c, d):
        """A quad, split in two when it is not flat."""
        pts = [self.vert(i) for i in (a, b, c, d)]
        n = cross(sub(pts[1], pts[0]), sub(pts[2], pts[0]))
        ln = math.sqrt(dot(n, n))
        off = abs(dot(sub(pts[3], pts[0]), n)) / ln if ln else 0
        if off > 1.5:
            self.face(colour, [a, b, c])
            self.face(colour, [a, c, d])
        else:
            self.face(colour, [a, b, c, d])

    def vert(self, i):
        k = 0
        for p in self.parts:
            if i < k + len(p["vertices"]):
                return p["vertices"][i - k]
            k += len(p["vertices"])
        raise IndexError(i)

    def rings(self, rings, colours, cap0=True, cap1=True, tip0=None, tip1=None):
        """Joins rings of equal length (lists of vertex indices) side by
        side; caps or closes the ends at a tip vertex. colours: one per
        band, or one for all."""
        n = len(rings[0])
        for r in range(len(rings) - 1):
            col = colours[r] if isinstance(colours, list) else colours
            for k in range(n):
                a, b = rings[r][k], rings[r][(k + 1) % n]
                c, d = rings[r + 1][(k + 1) % n], rings[r + 1][k]
                self.quad(col, a, b, c, d)
        c0 = colours[0] if isinstance(colours, list) else colours
        c1 = colours[-1] if isinstance(colours, list) else colours
        if tip0 is not None:
            for k in range(n):
                self.face(c0, [tip0, rings[0][(k + 1) % n], rings[0][k]])
        elif cap0:
            self.face(c0, rings[0][::-1])
        if tip1 is not None:
            for k in range(n):
                self.face(c1, [rings[-1][k], rings[-1][(k + 1) % n], tip1])
        elif cap1:
            self.face(c1, rings[-1])


def ring_at(mesh, centre, u, v, ru, rv, sides, turn=0.5):
    out = []
    for k in range(sides):
        a = 2 * math.pi * (k + turn) / sides
        out.append(mesh.v(add(centre, add(mul(u, ru * math.cos(a)), mul(v, rv * math.sin(a))))))
    return out


def tube(mesh, pts, side_axis, colours, sides=6, tip=None, cap0=True):
    """A tapered tube along pts [(centre, r_side, r_plane)]: r_side across
    side_axis, r_plane at right angles in the plane of the bend. Ends in a
    point at `tip` when given."""
    rings = []
    for i, (c, ru, rv) in enumerate(pts):
        if i == 0:
            t = sub(pts[1][0], c)
        elif i == len(pts) - 1:
            t = sub(c, pts[i - 1][0])
        else:
            t = add(norm(sub(c, pts[i - 1][0])), norm(sub(pts[i + 1][0], c)))
        t = norm(t)
        u = norm(sub(side_axis, mul(t, dot(side_axis, t))))
        v = cross(t, u)
        rings.append(ring_at(mesh, c, u, v, ru, rv, sides))
    tip_i = mesh.v(tip) if tip is not None else None
    mesh.rings(rings, colours, cap0=cap0, tip1=tip_i)


def sphere(mesh, centre, r, colour, seg=10, rows=6, squash=1.0):
    top = mesh.v(add(centre, [0, r * squash, 0]))
    rings = []
    for i in range(1, rows):
        a = math.pi * i / rows
        y = math.cos(a) * r * squash
        rr = math.sin(a) * r
        rings.append(
            [mesh.v(add(centre, [rr * math.cos(2 * math.pi * k / seg), y, -rr * math.sin(2 * math.pi * k / seg)])) for k in range(seg)]
        )
    bottom = mesh.v(add(centre, [0, -r * squash, 0]))
    n = seg
    for k in range(n):
        mesh.face(colour, [top, rings[0][k], rings[0][(k + 1) % n]])
    for r_ in range(len(rings) - 1):
        for k in range(n):
            mesh.quad(colour, rings[r_][k], rings[r_ + 1][k], rings[r_ + 1][(k + 1) % n], rings[r_][(k + 1) % n])
    for k in range(n):
        mesh.face(colour, [rings[-1][(k + 1) % n], rings[-1][k], bottom])


# ---- the shell ----

RIM_Y = 135
WIDE_Z = 72  # where the shell is widest: its front third
HALF_W = 212


def outline():
    """The carapace's margin seen from above, as (x, z, tooth) going round
    anticlockwise from the front middle (+z) toward +x; tooth marks a tooth
    tip (for the rim's darker shade)."""
    half = []
    # frontal margin between the eyes: two low lobes, then the eye notch
    half += [(0, 182, False), (22, 184, False), (42, 178, False), (54, 170, False), (66, 160, False), (84, 166, False)]
    # the anterolateral arc: an ellipse from the eye socket to the widest
    # point, five teeth on it (the first the eye socket's outer corner)
    a, b = HALF_W, 112
    start = math.acos(100 / a)  # angle of the outer orbital corner
    n = 8
    for i in range(n + 1):
        t = start - (start - 0.02) * i / n
        x, z = a * math.cos(t), WIDE_Z + b * math.sin(t)
        if i % 2 == 0:
            # a tooth tip, pushed out and forward
            nx, nz = math.cos(t) / a, math.sin(t) / b
            ln = math.hypot(nx, nz)
            x, z = x + 22 * nx / ln + 4, z + 22 * nz / ln + 8
            half.append((x, z, True))
        else:
            half.append((x - 6, z - 6, False))
    # posterolateral margin: tapering in to the back corner
    half += [(204, 40, False), (192, 0, False), (174, -45, False), (152, -90, False), (126, -132, False), (100, -162, False)]
    # the back edge
    half += [(70, -174, False), (36, -178, False), (0, -179, False)]
    full = half[:-1] + [(-x, z, t) for (x, z, t) in reversed(half[1:])]
    # anticlockwise seen from above is from +z toward +x
    return full


def shell(mesh):
    out = outline()
    cz = -10

    def scaled(s, y, smooth=0.0):
        """The outline toward the middle by s at height y, its teeth
        flattened by `smooth` (1: none left)."""
        ring = []
        m = len(out)
        for k, (x, z, t) in enumerate(out):
            if smooth and t:
                (x0, z0, _), (x1, z1, _) = out[k - 1], out[(k + 1) % m]
                x = x + ((x0 + x1) / 2 - x) * smooth
                z = z + ((z0 + z1) / 2 - z) * smooth
            ring.append(mesh.v([x * s, y, cz + (z - cz) * s]))
        return ring

    # dome from the rim up; the rim drawn slightly darker
    top = [
        scaled(1.0, RIM_Y),
        scaled(0.93, 158, 0.6),
        scaled(0.78, 184, 1.0),
        scaled(0.55, 203, 1.0),
        scaled(0.28, 212, 1.0),
    ]
    cap = mesh.v([0, 215, cz - 5])
    mesh.rings(top, ["shell_rim", "shell", "shell", "shell"], cap0=False, tip1=None, cap1=False)
    n = len(out)
    for k in range(n):
        mesh.face("shell", [top[-1][k], top[-1][(k + 1) % n], cap][::-1])
    # underside: the margin turns under, then the flat sternum
    under = [
        scaled(1.0, RIM_Y - 1),
        scaled(0.96, 120, 1.0),
        scaled(0.78, 106, 1.0),
        scaled(0.5, 101, 1.0),
    ]
    mesh.rings(under, ["belly", "belly", "belly"], cap0=False, cap1=False)
    mid = mesh.v([0, 100, cz])
    for k in range(n):
        mesh.face("belly", [under[-1][k], under[-1][(k + 1) % n], mid])
    # the rim's thin edge between the top and the underside rings
    for k in range(n):
        a, b = top[0][k], top[0][(k + 1) % n]
        c, d = under[0][(k + 1) % n], under[0][k]
        mesh.quad("shell_rim", a, d, c, b)


def abdomen(mesh):
    """The folded abdomen: a broad rounded flap flat under the sternum, one
    tile wide, in bands for its segments; its tip points forward."""
    # half-widths at stations from the back (z) toward the tip
    stations = [(-130, 76), (-100, 93), (-64, 100), (-28, 100), (6, 92), (36, 74), (56, 48), (68, 20)]
    ys = [97, 96, 97, 96, 97, 96, 97]
    rows = []
    for (z, w), y in zip(stations, ys + [96]):
        rows.append((z, w, y))
    for i in range(len(stations) - 1):
        z0, w0 = stations[i]
        z1, w1 = stations[i + 1]
        y = ys[i]
        col = "abdomen" if i % 2 == 0 else "abdomen_seam"
        a = mesh.v([w0, y, z0])
        b = mesh.v([w1, y, z1])
        c = mesh.v([-w1, y, z1])
        d = mesh.v([-w0, y, z0])
        # facing down
        mesh.face(col, [a, b, c, d])
    # the rounded tip
    t0 = mesh.v([20, 96, 68])
    t1 = mesh.v([0, 96, 76])
    t2 = mesh.v([-20, 96, 68])
    mesh.face("abdomen", [t0, t1, t2])


def face_bits(mesh):
    # mouthparts under the front edge: two dark plates
    for sx in (1, -1):
        pts = [[sx * 4, 128, 166], [sx * 30, 126, 162], [sx * 34, 112, 152], [sx * 6, 108, 156]]
        vs = [mesh.v(p) for p in pts]
        mesh.face("mouth", vs if sx < 0 else vs[::-1])
    # short antennae between the eyes, and the tiny antennules
    for sx in (1, -1):
        tube(mesh, [([sx * 16, 170, 172], 4, 4), ([sx * 24, 196, 200], 3, 3)], [1, 0, 0], "leg", sides=5, tip=[sx * 36, 222, 236])
        tube(mesh, [([sx * 6, 168, 176], 3, 3), ([sx * 8, 182, 192], 2, 2)], [1, 0, 0], "leg_dark", sides=4, tip=[sx * 10, 190, 206])


# ---- eyes ----

EYE_BASE = 50


def eye(mesh, sx):
    side = "l" if sx > 0 else "r"
    base = [sx * EYE_BASE, 168, 150]
    mesh.part(f"eye_{side}", "body", base)
    top = [sx * (EYE_BASE + 6), 228, 166]
    tube(mesh, [(base, 11, 11), (lerp(base, top, 0.5), 9, 9), (top, 10, 10)], [1, 0, 0], "stalk", sides=6)
    centre = [sx * (EYE_BASE + 8), 244, 170]
    sphere(mesh, centre, 22, "eye", seg=10, rows=6)
    # the pupil, a dark disc on the front, looking forward and a little out
    look = norm([sx * 0.25, 0.15, 1.0])
    c = add(centre, mul(look, 22.5))
    u = norm(cross([0, 1, 0], look))
    w = cross(look, u)
    ring = ring_at(mesh, c, u, w, 10, 11, 8)
    mesh.face("pupil", ring if sx < 0 else ring)
    # a tiny highlight: none (flat look)


# ---- claws ----

ARM_BASE = [118, 118, 112]
ELBOW = [200, 132, 176]
WRIST = [140, 152, 214]
PALM_X = 112
PALM_Z = 196  # where the palm starts, behind the wrist joint
JAW_Z = 352  # where the fingers part: the palm is 156 mm long, a third of the shell's width
FIXED_TOP = 135  # a held tile's underside rests here
GAP = 45  # the open fingers' gap at rest: a tile's thickness
FINGER_Z = 444  # the finger tips
ANCHOR_Z = JAW_Z + 112  # the held tile's middle: its near edge 12 mm past the jaw
FOLD = 22  # degrees the palms turn in toward the face when not holding


def palm_ring(mesh, sx, c, rx, top, bottom, flat, sides=10):
    """A swollen palm's section about c: rx across, `top` and `bottom` mm
    above and below c, its outer face rounder than its flatter inner face
    (`flat`, the inner side's share of rx), a teardrop deeper below."""
    out = []
    for k in range(sides):
        a = 2 * math.pi * (k + 0.5) / sides
        cx, cy = math.cos(a), math.sin(a)
        x = rx * cx * (1.0 if cx > 0 else flat)
        y = (top if cy > 0 else bottom) * cy
        # narrower toward the top edge: a teardrop
        if cy > 0:
            x *= 1 - 0.25 * cy
        # +x of the section is the claw's outer side; mirrored for the right
        out.append(mesh.v([sx * (c[0] + x), c[1] + y, c[2]]))
    return out


def claw(mesh, sx):
    side = "l" if sx > 0 else "r"
    X = lambda p: [sx * p[0], p[1], p[2]]
    up = [0, 1, 0]
    mesh.part(f"claw_{side}_arm", "body", X(ARM_BASE))
    # the merus: a stout three-sided-ish arm, out and forward
    tube(mesh, [(X(ARM_BASE), 22, 24), (X(lerp(ARM_BASE, ELBOW, 0.55)), 28, 32), (X(ELBOW), 26, 28)], up, "claw", sides=8)
    mesh.part(f"claw_{side}_wrist", f"claw_{side}_arm", X(ELBOW))
    # the carpus: a chunky knuckle back in toward the palm
    tube(mesh, [(X(sub(ELBOW, [8, 0, 6])), 28, 28), (X(lerp(ELBOW, WRIST, 0.45)), 36, 36), (X(WRIST), 30, 32)], up, "claw", sides=8)
    mesh.part(f"claw_{side}_palm", f"claw_{side}_wrist", X(WRIST))
    # the propodus: big, broad and swollen, deeper than wide
    # (z, centre y, half-width, above, below, inner flatness)
    rows = [
        (PALM_Z, 156, 36, 34, 36, 0.8),
        (PALM_Z + 22, 160, 54, 50, 54, 0.75),
        (PALM_Z + 62, 162, 62, 56, 62, 0.72),
        (PALM_Z + 108, 162, 60, 54, 60, 0.72),
        (JAW_Z - 14, 160, 52, 48, 54, 0.75),
        (JAW_Z, 158, 42, 42, 46, 0.8),
    ]
    rings = [palm_ring(mesh, sx, [PALM_X, y, z], rx, top, bottom, flat) for z, y, rx, top, bottom, flat in rows]
    mesh.rings(rings, "claw")
    # the fixed finger (pollex): stout, tapered, its top flat at FIXED_TOP
    tube(
        mesh,
        [
            (X([PALM_X, FIXED_TOP - 18, JAW_Z - 10]), 24, 18),
            (X([PALM_X, FIXED_TOP - 14, JAW_Z + 36]), 20, 14),
            (X([PALM_X, FIXED_TOP - 10, JAW_Z + 70]), 13, 10),
        ],
        [1, 0, 0],
        ["claw", "claw_tip"],
        sides=8,
        tip=X([PALM_X, FIXED_TOP - 4, FINGER_Z]),
    )
    # three blunt teeth on its biting edge
    def tooth(z, y, down):
        h = -7 if down else 7
        a = mesh.v(X([PALM_X - 8, y, z]))
        b = mesh.v(X([PALM_X + 8, y, z]))
        c = mesh.v(X([PALM_X, y + h, z + 9]))
        d = mesh.v(X([PALM_X, y, z + 18]))
        for f in ([a, c, b], [b, c, d], [d, c, a]):
            mesh.face("claw_tip", f)

    for z in (JAW_Z + 6, JAW_Z + 34, JAW_Z + 62):
        tooth(z, FIXED_TOP - 1, False)
    top = FIXED_TOP + GAP
    mesh.part(f"claw_{side}_finger", f"claw_{side}_palm", X([PALM_X, top + 20, JAW_Z - 14]))
    # the dactyl, hinged on top, curving down to its point
    tube(
        mesh,
        [
            (X([PALM_X, top + 20, JAW_Z - 14]), 22, 18),
            (X([PALM_X, top + 15, JAW_Z + 36]), 19, 14),
            (X([PALM_X, top + 11, JAW_Z + 70]), 12, 10),
        ],
        [1, 0, 0],
        ["claw", "claw_tip"],
        sides=8,
        tip=X([PALM_X, top + 3, FINGER_Z + 2]),
    )
    for z in (JAW_Z + 20, JAW_Z + 50):
        tooth(z, top + 1, True)


# ---- legs ----

LEG_HIP_Y = 115
# (hip x, hip z, sweep degrees forward of straight out, length scale)
LEGS = [
    (138, 52, 32, 0.92),
    (152, -2, 8, 1.0),
    (148, -62, -16, 1.0),
    (124, -118, -44, 0.9),
]
# the leg in its own plane, (out, up) from the hip: under the shell margin,
# up to the knee, down to the ankle, the tip on the sand
LEG_BEND = (86, 8)
LEG_KNEE = (150, 100)
LEG_ANKLE = (222, -34)
LEG_TIP = (244, -(LEG_HIP_Y - DROP))


def leg_frame(i, sx):
    hx, hz, sweep, k = LEGS[i]
    a = math.radians(sweep)
    d = [sx * math.cos(a), 0, math.sin(a)]
    hip = [sx * hx, LEG_HIP_Y, hz]
    n = cross(d, [0, 1, 0])  # positive turn about n lifts the leg's end
    return hip, d, n, k


def leg_point(i, sx, rp):
    hip, d, n, k = leg_frame(i, sx)
    r, y = rp
    return add(hip, add(mul(d, r * k), [0, y, 0]))


def leg(mesh, i, sx):
    side = "l" if sx > 0 else "r"
    hip, d, n, k = leg_frame(i, sx)
    P = lambda rp: leg_point(i, sx, rp)
    name = f"leg_{side}{i + 1}"
    mesh.part(f"{name}_hip", "body", hip)
    knee = P(LEG_KNEE)
    bend = P(LEG_BEND)
    knee_out = add(knee, mul(norm(sub(knee, bend)), 8))
    tube(
        mesh,
        # the merus flattened and broad: thin front to back, deep in its plane
        [(hip, 22, 22), (bend, 24, 26), (lerp(bend, knee, 0.5), 23, 34), (knee_out, 19, 23)],
        n,
        "leg",
        sides=6,
    )
    mesh.part(f"{name}_knee", f"{name}_hip", knee)
    ankle = P(LEG_ANKLE)
    knee_in = add(knee, mul(norm(sub(knee, ankle)), 8))
    tube(mesh, [(knee_in, 19, 20), (lerp(knee, ankle, 0.4), 18, 22), (ankle, 14, 15)], n, "leg", sides=6)
    mesh.part(f"{name}_foot", f"{name}_knee", ankle)
    tip = P(LEG_TIP)
    # the back pair's tips flattened into paddles, as a swimming crab's
    side_r, plane_r = (8, 24) if i == 3 else (13, 13)
    tube(
        mesh,
        [(add(ankle, mul(norm(sub(ankle, tip)), 6)), side_r + 2, plane_r), (lerp(ankle, tip, 0.45), side_r, plane_r * 0.8)],
        n,
        ["leg", "leg_dark"],
        sides=6,
        tip=tip,
    )


def leg_ik(i, sx, delta, body=(0, 0, 0), foot=0):
    """The hip, knee and foot turns that put leg i's tip `delta` mm from its
    rest place (in the crab's frame on the ground), with the body moved by
    `body`. A planar two-link solve: hip to knee, knee to tip."""
    hip, d, n, k = leg_frame(i, sx)
    tip0 = leg_point(i, sx, LEG_TIP)
    target = sub(add(tip0, list(delta)), list(body))
    rel = sub(target, hip)
    h = [rel[0], 0, rel[2]]
    r = math.hypot(h[0], h[2])
    y = rel[1]
    # swing about y from the rest direction to the target's
    swing = math.degrees(math.atan2(d[2], d[0]) - math.atan2(h[2], h[0]))
    # in the plane: link 1 hip->knee, link 2 knee->tip
    k1 = (LEG_KNEE[0] * k, LEG_KNEE[1])
    k2 = (LEG_TIP[0] * k - k1[0], LEG_TIP[1] - k1[1])
    l1, l2 = math.hypot(*k1), math.hypot(*k2)
    dist = min(math.hypot(r, y), l1 + l2 - 1)
    ang = math.atan2(y, r)
    c1 = (l1 * l1 + dist * dist - l2 * l2) / (2 * l1 * dist)
    a1 = ang + math.acos(max(-1, min(1, c1)))  # knee up
    kx, ky = l1 * math.cos(a1), l1 * math.sin(a1)
    a2 = math.atan2(y - ky, r - kx)
    lift = math.degrees(a1 - math.atan2(k1[1], k1[0]))
    flex = math.degrees(a2 - math.atan2(k2[1], k2[0])) - lift
    ry = rot_axis([0, 1, 0], swing)
    hip_r = matmul(ry, rot_axis(n, lift))
    return {
        "hip": turn_of(hip_r),
        "knee": turn_of(rot_axis(n, flex)),
        "foot": turn_of(rot_axis(n, foot)),
    }


# ---- animations ----

SIDES = [(1, "l"), (-1, "r")]


def pose(turn=None, move=None):
    p = {}
    if turn and any(turn):
        p["turn"] = turn
    if move and any(move):
        p["move"] = [round(m * SCALE) for m in move]
    return p


def legs_pose(deltas, body=(0, 0, 0), foot=None):
    """Poses of all eight legs; deltas(i, sx) -> the tip's offset."""
    out = {}
    for sx, s in SIDES:
        for i in range(4):
            f = foot(i, sx) if foot else 0
            ik = leg_ik(i, sx, deltas(i, sx), body, f)
            for joint in ("hip", "knee", "foot"):
                p = pose(ik[joint])
                out[f"leg_{s}{i + 1}_{joint}"] = p
    return out


def outward(i, sx, r, up=0.0):
    _, d, _, _ = leg_frame(i, sx)
    return (d[0] * r, up, d[2] * r)


def claws(**kw):
    """Claw poses: kw like l_arm=[x, y, z] turns; mirrored for r."""
    out = {}
    for key, t in kw.items():
        side, joint = key.split("_", 1)
        out[f"claw_{side}_{joint}"] = pose(t)
    return out


def mirror(t):
    """A left claw's turn for the right claw: y and z turns change sign."""
    return [t[0], -t[1], -t[2]]


def both(joint, t):
    return {f"l_{joint}": t, f"r_{joint}": mirror(t)}


def clean(parts):
    return {k: v for k, v in parts.items() if v}


def keys_of(frames):
    return [{"at": at, "parts": clean(parts)} for at, parts in frames]


GROUP_A = {(0, 1), (1, -1), (2, 1), (3, -1)}


def gait(ms, steps, axis, stride, lift, bob, sway=0):
    """A looping walk: legs in two alternating groups, each half the cycle
    on the ground moving back along `axis` (the crab moves forward along
    it), half in the air coming forward. axis is (x, z)."""
    frames = []
    for s in range(steps):
        u0 = s / steps
        body = (sway * math.sin(2 * math.pi * u0) * axis[0], -bob * (0.5 + 0.5 * math.cos(4 * math.pi * u0)), 0)

        def delta(i, sx, u0=u0):
            u = (u0 + (0 if (i, sx) in GROUP_A else 0.5)) % 1
            if u < 0.5:
                t = u / 0.5
                along = stride * (1 - 2 * t)
                up = 0
            else:
                t = (u - 0.5) / 0.5
                along = stride * (-1 + 2 * (0.5 - 0.5 * math.cos(math.pi * t)))
                up = lift * math.sin(math.pi * t)
            return (axis[0] * along, up, axis[1] * along)

        parts = legs_pose(delta, body)
        parts["body"] = pose(None, body)
        frames.append((round(ms * u0), parts))
    return frames


def animations():
    anims = {}
    still = lambda i, sx: (0, 0, 0)

    # idle: breathing bob, eye stalks twitch, claws open and close a little
    def idle_key(bob, eye_t, fing):
        body = (0, -bob, 0)
        parts = legs_pose(still, body)
        parts["body"] = pose(None, body)
        parts["eye_l"] = pose(eye_t[0])
        parts["eye_r"] = pose(eye_t[1])
        parts.update(claws(**both("finger", [fing, 0, 0]), **both("palm", [0, -FOLD, 0])))
        return parts

    anims["idle"] = {
        "ms": 2400,
        "keys": keys_of(
            [
                (0, idle_key(0, ([0, 0, 0], [0, 0, 0]), 16)),
                (600, idle_key(4, ([-6, 8, 0], [0, 0, 0]), 10)),
                (900, idle_key(5, ([-6, 8, 0], [4, -10, 0]), 8)),
                (1200, idle_key(4, ([0, 0, 0], [4, -10, 0]), 14)),
                (1800, idle_key(1, ([0, 0, 0], [0, 0, 0]), 18)),
            ]
        ),
    }

    # walk sideways: toward the crab's left (+x); played backwards it goes right
    anims["walk_side"] = {"ms": 480, "ease": "linear", "keys": keys_of(gait(480, 12, (1, 0), 55, 50, 10, sway=0))}
    # walk forward: a slower shuffle toward +z; backwards it backs up
    anims["walk_fwd"] = {"ms": 800, "ease": "linear", "keys": keys_of(gait(800, 12, (0, 1), 38, 34, 6))}
    # claws ride a little with the walk
    for name, ms, a in (("walk_side", 480, 4), ("walk_fwd", 800, 3)):
        for key in anims[name]["keys"]:
            u = key["at"] / ms
            key["parts"].update(clean(claws(**both("arm", [round(a * math.sin(4 * math.pi * u)), 0, 0]), **both("palm", [0, -FOLD, 0]))))

    # jump: crouch, spring, a floaty hang with the legs tucked then spread,
    # reach down, land. In place: the game lifts the crab.
    def jump_key(body, legs, eyes=0, arm=0, fing=12, foot=0):
        parts = legs_pose(legs, body, foot=lambda i, sx: foot)
        parts["body"] = pose(None, body)
        parts.update(claws(**both("arm", [arm, 0, 0]), **both("finger", [fing, 0, 0]), **both("palm", [0, -FOLD, 0])))
        parts["eye_l"] = pose([eyes, 0, 0])
        parts["eye_r"] = pose([eyes, 0, 0])
        return parts

    anims["jump"] = {
        "ms": 1300,
        "keys": keys_of(
            [
                (0, jump_key((0, 0, 0), still)),
                (140, jump_key((0, -48, 0), still, eyes=10, arm=8)),
                (260, jump_key((0, 30, 0), still, eyes=-14, arm=-10)),
                (420, jump_key((0, 30, 0), lambda i, sx: outward(i, sx, -40, 120), eyes=-8, arm=-14, fing=4, foot=-20)),
                (760, jump_key((0, 30, 0), lambda i, sx: outward(i, sx, 70, 70), eyes=0, arm=-20, fing=-6, foot=10)),
                (1020, jump_key((0, 20, 0), lambda i, sx: outward(i, sx, 30, 0), eyes=4, arm=-6, fing=6)),
                (1120, jump_key((0, -40, 0), still, eyes=12, arm=10, fing=16)),
                (1300, jump_key((0, 0, 0), still, fing=16)),
            ]
        ),
    }

    # grabs: that claw reaches forward and down, opens, closes, lifts
    def grab(side):
        s = side
        m = (lambda t: t) if s == "l" else mirror
        k = lambda **kw: {f"claw_{s}_{j}": pose(m(t)) for j, t in kw.items()}
        return {
            "ms": 900,
            "keys": keys_of(
                [
                    (0, k(arm=[0, 0, 0], wrist=[0, 0, 0], palm=[0, -FOLD, 0], finger=[12, 0, 0])),
                    (260, k(arm=[33, -8, 0], wrist=[0, 8, 0], palm=[-33, 0, 0], finger=[-22, 0, 0])),
                    (420, k(arm=[54, -8, 0], wrist=[0, 8, 0], palm=[-54, 0, 0], finger=[-24, 0, 0])),
                    (560, k(arm=[54, -8, 0], wrist=[0, 8, 0], palm=[-54, 0, 0], finger=[0, 0, 0])),
                    (900, k(arm=[0, 0, 0], wrist=[0, 0, 0], palm=[0, 0, 0], finger=[0, 0, 0])),
                ]
            ),
        }

    anims["grab_left"] = grab("l")
    anims["grab_right"] = grab("r")

    # hold: both claws raised in front, fingers on the tiles
    hold = claws(**both("arm", [-24, 0, 0]), **both("wrist", [0, 0, 0]), **both("palm", [16, 0, 0]), **both("finger", [0, 0, 0]))
    hold2 = claws(**both("arm", [-27, 0, 0]), **both("wrist", [0, 0, 0]), **both("palm", [18, 0, 0]), **both("finger", [0, 0, 0]))
    anims["hold"] = {"ms": 1600, "keys": keys_of([(0, hold), (800, hold2)])}

    # places: from holding, that claw lowers forward, opens, lifts back
    def place(side):
        s = side
        m = (lambda t: t) if s == "l" else mirror
        k = lambda **kw: {f"claw_{s}_{j}": pose(m(t)) for j, t in kw.items()}
        return {
            "ms": 800,
            "keys": keys_of(
                [
                    (0, k(arm=[-24, 0, 0], palm=[16, 0, 0], finger=[0, 0, 0])),
                    (280, k(arm=[33, -8, 0], wrist=[0, 8, 0], palm=[-33, 0, 0], finger=[0, 0, 0])),
                    (420, k(arm=[33, -8, 0], wrist=[0, 8, 0], palm=[-33, 0, 0], finger=[-26, 0, 0])),
                    (800, k(arm=[0, 0, 0], wrist=[0, 0, 0], palm=[0, -FOLD, 0], finger=[14, 0, 0])),
                ]
            ),
        }

    anims["place_left"] = place("l")
    anims["place_right"] = place("r")
    return anims


def build():
    mesh = Mesh()
    mesh.part("body", None, [0, 150, 0])
    shell(mesh)
    abdomen(mesh)
    face_bits(mesh)
    for sx, _ in SIDES:
        eye(mesh, sx)
    for sx, _ in SIDES:
        claw(mesh, sx)
    for sx, _ in SIDES:
        for i in range(4):
            leg(mesh, i, sx)
    model = {
        "_note": (
            "The crab every player steers, its shell in the player's colour (SECOND-ORDER R1.90). "
            "Anatomy of a green or blue shore crab, checked from above, the side, the front and 3/4: "
            "a carapace wider than long (262 mm across the teeth, 249 behind them, 199 long; about 1.25 tiles across), widest across its front third just behind the eyes, "
            "five teeth on each front-side margin, tapering to a narrower back edge, gently domed; "
            "two eyes on short stalks in notches at the front middle, short antennae between them; "
            "two claws from the front corners under the shell, each an arm (merus), a wrist (carpus) and a big palm "
            "(propodus, 86 mm long, 65 deep and up to 61 broad, a swollen teardrop section; the heaviest limbs) with a stout toothed fixed finger below and a movable finger (dactyl) hinged on top, folded in front of the face with the palms turned 22 degrees in (idle, walks, jump; hold, grab and place turn them straight so a held tile sits square); "
            "four pairs of walking legs from under the sides, splayed sideways (about 430 mm tip to tip), the upper segment (merus) flattened and broad, each rising from under the margin to a high knee "
            "above the shell and angling down to a pointed tip, bending in the plane across the body, the back pair swept back with paddle-flat tips as a swimming crab's; "
            "the abdomen a broad rounded flap folded flat under the body, about 110 mm wide. A held tile keeps its full 200 mm size, a big load for the crab. "
            "Parts: body; eye_l, eye_r; claw_l_arm, claw_l_wrist, claw_l_palm, claw_l_finger and claw_r_...; "
            "leg_l1_hip, leg_l1_knee, leg_l1_foot for left legs 1 (front) to 4 (back) and leg_r1_... on the right, each named after "
            "the joint it turns about (hip: coxa and merus up to the knee; knee: carpus and propodus down to the ankle; foot: the dactyl). "
            "Anchors claw_left and claw_right: the middle of a held tile's underside, the tile's near edge between the open fingers "
            "(a 25 mm gap at rest). The walks go toward +x (walk_side) and +z (walk_fwd); played backwards they go the other way. "
            "The jump is the crab's pose only; the game lifts it. Written by tools/models/crab.py."
        ),
        "name": "Crab",
        "kind": "crab",
        "unit": "mm",
        "view": {"yaw": 30, "pitch": 30},
        "colours": {
            "shell": {"slot": "player", "shade": 100},
            "shell_rim": {"slot": "player", "shade": 90},
            "belly": {"slot": "player", "shade": 118},
            "abdomen": {"slot": "player", "shade": 165},
            "abdomen_seam": {"slot": "player", "shade": 140},
            "claw": {"slot": "player", "shade": 108},
            "claw_tip": {"slot": "player", "shade": 40},
            "leg": {"slot": "player", "shade": 114},
            "leg_dark": {"slot": "player", "shade": 60},
            "stalk": {"slot": "player", "shade": 110},
            "eye": [246, 246, 238],
            "pupil": [24, 22, 30],
            "mouth": {"slot": "player", "shade": 50},
        },
        "parts": mesh.parts,
        "faces": mesh.faces,
        "anchors": {
            "claw_left": {"part": "claw_l_palm", "at": out_mm([PALM_X, FIXED_TOP, ANCHOR_Z])},
            "claw_right": {"part": "claw_r_palm", "at": out_mm([-PALM_X, FIXED_TOP, ANCHOR_Z])},
        },
        "animations": animations(),
    }
    return model


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
                head = f'"name": {json.dumps(p["name"])}, '
                if "parent" in p:
                    head += f'"parent": {json.dumps(p["parent"])}, '
                parts.append(f'    {{{head}"pivot": {json.dumps(p["pivot"])}, "vertices": [\n        {vs}\n    ]}}')
            lines.append('  "parts": [\n' + ",\n".join(parts) + f"\n  ]{comma}")
        elif k == "faces":
            fs = ",\n    ".join(json.dumps(f) for f in v)
            lines.append(f'  "faces": [\n    {fs}\n  ]{comma}')
        elif k == "animations":
            items = []
            for name_, a in v.items():
                head = {kk: vv for kk, vv in a.items() if kk != "keys"}
                ks = ",\n        ".join(json.dumps(kk) for kk in a["keys"])
                items.append(f'    {json.dumps(name_)}: {json.dumps(head)[:-1]}, "keys": [\n        {ks}\n    ]}}')
            lines.append('  "animations": {\n' + ",\n".join(items) + f"\n  }}{comma}")
        elif isinstance(v, dict) and k in ("anchors", "colours"):
            inner = ",\n    ".join(f"{json.dumps(a)}: {json.dumps(b)}" for a, b in v.items())
            lines.append(f"  {json.dumps(k)}: {{\n    {inner}\n  }}{comma}")
        else:
            lines.append(f"  {json.dumps(k)}: {json.dumps(v)}{comma}")
    lines.append("}")
    path.write_text("\n".join(lines) + "\n")
    print(path.relative_to(ROOT), len(model["faces"]), "faces", sum(len(p["vertices"]) for p in model["parts"]), "vertices")


if __name__ == "__main__":
    write("crab", build())
