//! How a loose tile tumbles (PLAN.md, gameplay 9; SECOND-ORDER R2.3, R8.*).
//!
//! A tile awake is a light rigid box with a velocity and a spin. Each tick
//! it finds what it touches: the points of its faces ([`samples`]) that lie
//! on the ground or on another tile's face.
//!
//! - Touching nothing, it falls, turning as it spins (water slows the
//!   spin).
//! - Touching, with its middle over what it touches, it rests on it: its
//!   spin stops, and it slides down its own face when that is steeper than
//!   `tiles.repose_deg` (friction slows it when flatter).
//! - Touching, with its middle past the edge of what it touches (it landed
//!   on an edge or a corner, or off-centre on another tile, or what held
//!   one side was pulled), it tips over that edge, faster the further its
//!   middle overhangs, until another part of it lands. So tiles roll off
//!   pile edges, and a tile on its side falls onto a face.
//!
//! It never moves into the side of another tile or up a wall of the ground
//! (it stops), and never sinks into what holds it (it is lifted out). A
//! tile turned past upright is the same box the other way up
//! (`face_down` flips). Still for [`QUIET_TICKS`] ticks it sleeps, but
//! only with its middle held: over what it touches, or leaning on the side
//! of a tile (R16.3); awake for `tiles.settle_ms` (twice that while still
//! tipping over an edge) it is dropped straight down onto what is under it
//! and sleeps.
//!
//! All integer, in id order: the same on every machine, so a replayed
//! `Match` tumbles the same way. Online every browser runs it and the
//! host's resting poses win (`sync.rs`).

use crate::gfx::fx::{self, mul, ONE};
use crate::gfx::geom::{v3, Affine, V3};
use crate::model::mm1;

use super::ground::Ground;
use super::tiles::{face_height, Physics, State, TileId, Tiles, REST_GAP_MM, SIZE_MM, THICK_MM};

/// A point this close over what holds it touches it.
const CONTACT_MM: i32 = 2;
/// A point is lifted onto a tile's face only when it was no more than
/// this under it before it moved.
const LIFT_EPS: i32 = 16;
/// A middle that overhangs what it touches by less than this is balanced.
const BALANCE_MM: i32 = 3;
/// Contacts this close to the nearest one along the overhang make the edge
/// a tile tips over.
const EDGE_MM: i32 = 10;
/// Slower than this (mm/s) and moving less than [`STILL_UM`] in a tick, a
/// tile touching something is still.
const STILL_MM_S: i32 = 8;
const STILL_UM: i32 = 400;
/// A tile touching something whose middle moves less than this many mm
/// in this many ticks sleeps (a creep, not a slide).
pub const CREEP_MM: i32 = 3;
pub const CREEP_TICKS: u32 = 25;
/// Still this many ticks running, it sleeps.
pub const QUIET_TICKS: u8 = 3;
/// The share of its spin a tile in the water loses each tick, and the
/// share of its speed across it keeps when it lands.
const SPIN_DRAG_PCT: i32 = 3;
const LAND_KEEP_PCT: i32 = 50;
/// A tile landing faster than this (mm/s) hops back up with this share of
/// its speed: a heavy thud and a small hop, a few millimetres (R16.10).
const BOUNCE_FROM_MM_S: i32 = 600;
const BOUNCE_PCT: i32 = 12;
/// How fast friction stops a resting tile's twist, radians a second
/// each second.
const TWIST_STOP: i32 = 15;
/// The share of the friction of a tile at rest that a sliding one feels.
const SLIDING_PCT: i32 = 80;
/// The share of its speed a sliding tile gives a loose tile it runs into.
const KNOCK_PCT: i32 = 60;
/// Slower than this (mm/s) a tile running into another does not knock it
/// (R16.6).
const KNOCK_MIN_MM_S: i32 = 20;
/// The fastest it spins, radians a second.
const SPIN_MAX: i32 = 12;
/// How a box's weight is spread for turning about its middle: (side² +
/// thickness²) / 12, Q16 square metres.
const SPREAD: i32 = ((SIZE_MM as i64 * SIZE_MM as i64 + THICK_MM as i64 * THICK_MM as i64) * ONE as i64
    / 12
    / 1_000_000) as i32;

/// How many points of a tile the physics samples.
pub const SAMPLES: usize = 42;

/// The points of a box frame the physics samples: a 5 × 5 grid over its
/// underside (50 mm apart), the corners, edge middles and middle of its
/// top, and the corners and edge middles half way up its sides (so it
/// meets a tile lying level beside it).
pub fn samples(p: &Affine) -> [V3; SAMPLES] {
    let q = mm1(SIZE_MM / 4);
    let h = mm1(SIZE_MM / 2);
    let mut out = [V3::ZERO; SAMPLES];
    let mut k = 0;
    for a in -2..=2 {
        for b in -2..=2 {
            out[k] = p.apply(v3(a * q, 0, b * q));
            k += 1;
        }
    }
    for a in [-h, 0, h] {
        for b in [-h, 0, h] {
            out[k] = p.apply(v3(a, mm1(THICK_MM), b));
            k += 1;
            if a != 0 || b != 0 {
                out[k] = p.apply(v3(a, mm1(THICK_MM / 2), b));
                k += 1;
            }
        }
    }
    out
}

/// `p` turned by `angle` (65536 a turn) about the unit axis `k` through
/// `pivot`.
pub fn turn_about(p: &Affine, pivot: V3, k: V3, angle: i32) -> Affine {
    let (s, c) = (fx::sin(angle), fx::cos(angle));
    // Rodrigues: v c + (k × v) s + k (k · v)(1 - c)
    let r = |v: V3| {
        v.scale(c)
            .add(k.cross(v).scale(s))
            .add(k.scale(mul(k.dot(v), ONE - c)))
    };
    let mut out = Affine {
        c0: r(p.c0),
        c1: r(p.c1),
        c2: r(p.c2),
        t: pivot.add(r(p.t.sub(pivot))),
    };
    square_up(&mut out);
    out
}

/// Makes a frame's columns unit and square to each other again, keeping
/// its up (`c1`) and as much of `c0` as it can.
fn square_up(p: &mut Affine) {
    let n = p.c1.norm();
    let mut c0 = p.c0.sub(n.scale(p.c0.dot(n))).norm();
    if c0 == V3::ZERO {
        c0 = n.cross(v3(0, 0, ONE)).norm();
    }
    p.c1 = n;
    p.c0 = c0;
    p.c2 = c0.cross(n).norm();
}

/// Radians (Q16) as 65536 a turn.
fn to_turn(rad: i32) -> i32 {
    (rad as i64 * fx::TURN as i64 / fx::TWO_PI) as i32
}

/// A point in a box frame's own axes (the frame square and unit).
fn local(p: &Affine, q: V3) -> V3 {
    let d = q.sub(p.t);
    v3(d.dot(p.c0), d.dot(p.c1), d.dot(p.c2))
}

/// Whether a point is deep inside a box: inside its square, more than
/// [`CONTACT_MM`] inside its faces.
fn deep_in(p: &Affine, q: V3) -> bool {
    let half = mm1(SIZE_MM / 2 - CONTACT_MM);
    if (q.x - p.t.x).abs() > half * 3 / 2 || (q.z - p.t.z).abs() > half * 3 / 2 {
        return false;
    }
    let l = local(p, q);
    l.x.abs() < half && l.z.abs() < half && l.y > mm1(CONTACT_MM) && l.y < mm1(THICK_MM - CONTACT_MM)
}

/// The edge a tile tips over, if its middle `m` overhangs every point it
/// touches by more than [`BALANCE_MM`] along one of a few directions
/// across (its own axes and their diagonals, and the world's every 22.5
/// degrees): the axis
/// it turns about (the overhanging side going down), the edge's middle,
/// and how far the middle overhangs it.
fn overhang(m: V3, p: &Affine, contacts: &[V3]) -> Option<(V3, V3, i32)> {
    // the world's every 22.5 degrees too, so a middle just past a corner
    // of what it touches, at any heading, is found (R16.3)
    let (c, s) = (ONE * 924 / 1000, ONE * 383 / 1000);
    let h = ONE * 707 / 1000;
    let dirs = [
        p.c0,
        p.c1,
        p.c2,
        p.c0.add(p.c2),
        p.c0.sub(p.c2),
        v3(ONE, 0, 0),
        v3(0, 0, ONE),
        v3(h, 0, h),
        v3(h, 0, -h),
        v3(c, 0, s),
        v3(s, 0, c),
        v3(c, 0, -s),
        v3(s, 0, -c),
    ];
    let mut best: Option<(i32, V3)> = None;
    for d in dirs {
        let d = d.flat();
        if !d.longer_than(ONE / 8) {
            continue;
        }
        let d = d.norm();
        for u in [d, d.neg()] {
            let near = contacts.iter().map(|q| q.sub(m).flat().dot(u)).max()?;
            let r = -near;
            if r > mm1(BALANCE_MM) && best.is_none_or(|b| r > b.0) {
                best = Some((r, u));
            }
        }
    }
    let (r, u) = best?;
    let (mut sum, mut n) = (V3::ZERO, 0);
    for q in contacts {
        if q.sub(m).flat().dot(u) >= -r - mm1(EDGE_MM) {
            sum = sum.add(*q);
            n += 1;
        }
    }
    let pivot = v3(sum.x / n, sum.y / n, sum.z / n);
    Some((V3::UP.cross(u).norm(), pivot, r))
}

/// A velocity along a face whose up is `n`, one tick of `dt_ms` on: gravity
/// down the face (when the face is what lies on its support: `on_face`),
/// less friction (wood on wood): a tile still starts to slide only when
/// steeper than `tiles.repose_deg`, and one sliding feels a little less
/// ([`SLIDING_PCT`]), so it slides on rather than creeping. At most
/// `tiles.slide_mm_per_s`.
fn slide(v: V3, n: V3, on_face: bool, ph: &Physics, dt_ms: i32) -> V3 {
    let per = |a: i32| (a as i64 * dt_ms as i64 / 1000) as i32;
    let cos = n.y.max(0);
    let sin = n.flat().len();
    let moving = v.longer_than(mm1(STILL_MM_S));
    let mu = if moving {
        ph.repose * SLIDING_PCT / 100
    } else {
        ph.repose
    };
    let mut a = mul(ph.gravity, sin - mul(mu, cos));
    if !moving && a > 0 {
        // it breaks loose: a clear start, not a creep
        a = a.max(mul(ph.gravity, ONE / 10));
    }
    if !on_face {
        // on its edge: only friction
        a = -mul(ph.gravity, mul(ph.repose, cos.max(ONE / 2)));
    }
    let down = n.scale(n.y).sub(V3::UP);
    let v = if a > 0 && down.longer_than(ONE / 64) {
        v.add(down.with_len(per(a)))
    } else {
        let left = v.len() - per(-a.min(0));
        if left <= 0 {
            V3::ZERO
        } else {
            v.with_len(left)
        }
    };
    v.clamp_len(ph.slide)
}

impl Tiles {
    /// A crab whose body (round, `radius` about its middle over its feet
    /// `at`, as high as it is wide) runs into a loose tile that nothing
    /// lies on pushes it away: the tile wakes and moves off fast enough
    /// to take `pct` of the overlap away in a tick of `dt_ms`, twisting if
    /// it was pushed off its middle; it slides on, slowed by friction,
    /// and stops at what it meets. A tile the crab stands on (its top no
    /// more than `step` over the feet) or one with others on it (they
    /// hold it down) is not pushed. The push also makes up what the sand's
    /// friction takes from it in the tick (R16.7), so a heavier tile is
    /// pushed as far. Returns the tiles it pushed.
    pub fn nudge(
        &mut self,
        at: V3,
        radius: i32,
        step: i32,
        pct: i32,
        ph: &Physics,
        dt_ms: i32,
    ) -> Vec<TileId> {
        let grip = (mul(ph.gravity, ph.repose * SLIDING_PCT / 100) as i64 * dt_ms as i64 / 1000) as i32;
        let reach = radius + mm1(SIZE_MM * 3 / 4);
        let mut pushed = Vec::new();
        for i in 0..self.all.len() {
            let t = &self.all[i];
            if !t.is_loose() || (t.pose.t.x - at.x).abs() > reach || (t.pose.t.z - at.z).abs() > reach {
                continue;
            }
            let pts = samples(&t.pose);
            let lo = pts.iter().map(|q| q.y).min().unwrap_or(0);
            let hi = pts.iter().map(|q| q.y).max().unwrap_or(0);
            if hi <= at.y + step || lo >= at.y + 2 * radius {
                continue;
            }
            // the nearest point of its outline from above to the crab's
            // middle, in its own axes laid flat
            let (e0, e2) = (t.pose.c0.flat(), t.pose.c2.flat());
            let half = mm1(SIZE_MM / 2);
            let (h0, h2) = (mul(half, e0.len()), mul(half, e2.len()));
            let (e0, e2) = (e0.norm(), e2.norm());
            let d = at.sub(t.pose.t).flat();
            let u = d.dot(e0).clamp(-h0, h0);
            let w = d.dot(e2).clamp(-h2, h2);
            let hit = t.pose.t.flat().add(e0.scale(u)).add(e2.scale(w));
            let off = hit.sub(at.flat());
            if off.longer_than(radius) {
                continue;
            }
            let dir = if off.longer_than(mm1(1)) {
                off.norm()
            } else {
                t.pose.t.sub(at).flat().norm()
            };
            if dir == V3::ZERO {
                continue;
            }
            let id = t.id;
            if self.all.iter().any(|o| o.is_loose() && o.rests_on.contains(&id)) {
                continue;
            }
            // a crab only just touching it (half a millimetre) does not
            // push it, so a tile a crab stands against comes to rest
            let deep = radius - off.len();
            if deep < mm1(1) / 2 {
                continue;
            }
            let speed = (deep as i64 * pct as i64 * 10 / dt_ms.max(1) as i64) as i32 + grip;
            let t = &mut self.all[i];
            if t.state == (State::Loose { awake: false }) {
                t.state = State::Loose { awake: true };
                t.awake_ticks = 0;
                t.stop();
            }
            let along = t.vel.dot(dir);
            if along < speed {
                t.vel = t.vel.add(dir.scale(speed - along));
            }
            // pushed off its middle, it twists: the push's turn about its
            // middle over how its weight is spread, halved for the sand's
            // grip
            let lever = hit.sub(t.pose.t.flat());
            let turn = lever.cross(dir.scale(speed)).y;
            let w = (turn as i64 * ONE as i64 / SPREAD.max(1) as i64 / 2) as i32;
            let twist = V3::UP.scale(w.clamp(-SPIN_MAX * ONE / 2, SPIN_MAX * ONE / 2));
            if t.spin.dot(V3::UP).abs() < twist.y.abs() {
                t.spin = twist;
            }
            t.quiet = 0;
            pushed.push(id);
        }
        pushed
    }

    /// One tick of `dt_ms` for every tumbling tile, in id order. Returns
    /// the tiles that came to rest.
    pub fn step(&mut self, ground: &Ground, ph: &Physics, dt_ms: i32) -> Vec<TileId> {
        let mut settled = Vec::new();
        for i in 0..self.all.len() {
            if self.all[i].state == (State::Loose { awake: true }) && self.tumble(ground, ph, dt_ms, i) {
                settled.push(self.all[i].id);
            }
        }
        settled
    }

    /// The tiles (not held, not `i`) near enough tile `i` to touch it.
    fn near(&self, i: usize) -> Vec<usize> {
        let me = self.all[i].pose.t;
        let r = mm1(SIZE_MM * 3 / 2);
        (0..self.all.len())
            .filter(|&j| j != i && !self.all[j].is_held())
            .filter(|&j| {
                let t = self.all[j].pose.t;
                (t.x - me.x).abs() < r && (t.z - me.z).abs() < r
            })
            .collect()
    }

    /// What holds up the point `q`: the ground, or the highest face that
    /// looks up of a tile `near` that is under it (not more than `gap`
    /// over it). A face that looks down (the underside of a tile above)
    /// holds nothing up.
    fn support_near(&self, ground: &Ground, near: &[usize], q: V3, gap: i32) -> i32 {
        let reach = mm1(SIZE_MM * 3 / 4);
        let mut s = ground.height(q.x, q.z);
        for &j in near {
            let p = &self.all[j].pose;
            if (p.t.x - q.x).abs() > reach || (p.t.z - q.z).abs() > reach {
                continue;
            }
            if let Some(h) = face_height(p, q.x, q.z, p.c1.y >= 0) {
                if h <= q.y + gap && h > s {
                    s = h;
                }
            }
        }
        s
    }

    /// How far the points `new` (that were `old`) are under what holds
    /// them, at most: the ground, or a tile's face that the point was over
    /// before (so a tile is never lifted onto a face it was beside, and two
    /// tiles leaning on each other never lift each other up).
    fn sunk(&self, ground: &Ground, near: &[usize], old: &[V3; SAMPLES], new: &[V3; SAMPLES]) -> i32 {
        old.iter()
            .zip(new)
            .map(|(o, q)| self.support_near(ground, near, v3(q.x, o.y.max(q.y), q.z), LIFT_EPS) - q.y)
            .max()
            .unwrap_or(0)
    }

    /// Whether the points moving from `old` to `new` run into a wall: the
    /// ground rising past where a point was by more than [`REST_GAP_MM`],
    /// or the side of another tile.
    fn walled(&self, ground: &Ground, near: &[usize], old: &[V3; SAMPLES], new: &[V3; SAMPLES]) -> bool {
        self.wall(ground, near, old, new).is_some()
    }

    /// What the points moving from `old` to `new` run into first, if
    /// anything: the ground (`None` inside), or the tile whose side it is.
    fn wall(
        &self,
        ground: &Ground,
        near: &[usize],
        old: &[V3; SAMPLES],
        new: &[V3; SAMPLES],
    ) -> Option<Option<usize>> {
        let gap = mm1(REST_GAP_MM);
        for (o, q) in old.iter().zip(new) {
            let g = ground.height(q.x, q.z);
            if g > o.y.max(q.y) + gap && g > ground.height(o.x, o.z) {
                return Some(None);
            }
            for &j in near {
                if deep_in(&self.all[j].pose, *q) && !deep_in(&self.all[j].pose, *o) {
                    return Some(Some(j));
                }
            }
        }
        None
    }

    /// Tile `j` is knocked by another running into its side at `v`: if it
    /// is loose and nothing lies on it, it wakes and moves off at `v`
    /// (unless it is moving that way faster already).
    fn knock(&mut self, j: usize, v: V3) {
        let id = self.all[j].id;
        if !self.all[j].is_loose() || self.all.iter().any(|o| o.is_loose() && o.rests_on.contains(&id)) {
            return;
        }
        let t = &mut self.all[j];
        if t.state == (State::Loose { awake: false }) {
            t.state = State::Loose { awake: true };
            t.awake_ticks = 0;
            t.stop();
        }
        let dir = v.norm();
        let along = t.vel.dot(dir);
        let speed = v.len();
        if along < speed {
            t.vel = t.vel.add(dir.scale(speed - along));
        }
        t.quiet = 0;
    }

    /// One tick of tile `i`; whether it came to rest.
    fn tumble(&mut self, ground: &Ground, ph: &Physics, dt_ms: i32, i: usize) -> bool {
        let per = |v: i32| (v as i64 * dt_ms as i64 / 1000) as i32;
        let near = self.near(i);
        let (pose0, mut vel, mut spin, middle) = {
            let t = &self.all[i];
            (t.pose, t.vel, t.spin, t.middle())
        };
        let old = samples(&pose0);
        // how fast it came into this tick: what a knock passes on (not
        // what gravity adds this tick, so a tile pressed against another
        // by its slope does not knock it again and again)
        let came = v3(vel.x, 0, vel.z);
        // what it touches now
        let mut touch = [V3::ZERO; SAMPLES];
        let mut n = 0;
        for q in &old {
            if self.support_near(ground, &near, *q, mm1(CONTACT_MM)) - q.y >= -mm1(CONTACT_MM) {
                touch[n] = *q;
                n += 1;
            }
        }
        let touch = &touch[..n];
        let flying = touch.is_empty();
        let mut tip = None;
        if flying {
            vel.y = (vel.y - per(ph.gravity)).max(-ph.fall_max);
            spin = spin.scale(ONE - ONE * SPIN_DRAG_PCT / 100);
        } else {
            let up = pose0.c1;
            // what it moved into it is stopped by; what is left runs along
            // its face
            let into = vel.dot(up);
            if into < 0 {
                // and friction takes from what is left as much as the
                // knock into it allows (wood on wood)
                let along = vel.sub(up.scale(into));
                let left = along.len() - mul(ph.repose, -into);
                vel = if left <= 0 { V3::ZERO } else { along.with_len(left) };
            }
            tip = overhang(middle, &pose0, touch);
            // on its face: resting, or tipping over a support's edge under
            // its underside (not on an edge of its own)
            let on_face = tip.is_none_or(|(_, pivot, _)| {
                let l = local(&pose0, pivot);
                let inner = mm1(SIZE_MM / 2 - EDGE_MM);
                l.x.abs() < inner && l.z.abs() < inner && l.y < mm1(CONTACT_MM)
            });
            vel = slide(vel, up, on_face, ph, dt_ms);
            spin = match tip {
                Some((k, _, r)) => {
                    let alpha = (ph.gravity as i64 * r as i64 / (SPREAD + mul(r, r)).max(1) as i64) as i32;
                    let w = (spin.dot(k).max(0) + per(alpha)).min(SPIN_MAX * ONE);
                    k.scale(w)
                }
                None => {
                    // resting: only a twist about its up (a nudge off its
                    // middle), which friction stops
                    let w = spin.dot(up);
                    let left = w.abs() - per(TWIST_STOP * ONE);
                    if left <= 0 {
                        V3::ZERO
                    } else {
                        up.scale(left * w.signum())
                    }
                }
            };
        }
        // turn
        let mut pose = pose0;
        let mut held_back = false;
        // a tip stopped by the side of a tile (it leans on it), not by
        // landing on something
        let mut leans = false;
        if spin != V3::ZERO {
            let full = to_turn(per(spin.len()));
            let axis = spin.norm();
            let pivot = tip.map_or(middle, |t| t.1);
            let mut a = full;
            let mut turned = false;
            for _ in 0..4 {
                let cand = turn_about(&pose0, pivot, axis, a);
                let new = samples(&cand);
                let sinks = tip.is_some() && self.sunk(ground, &near, &old, &new) > mm1(CONTACT_MM);
                let walled = self.walled(ground, &near, &old, &new);
                if !sinks && !walled {
                    pose = cand;
                    turned = true;
                    break;
                }
                leans = walled && !sinks;
                a /= 2;
            }
            if !turned || a != full {
                // it turned onto something: the turn stops there
                spin = V3::ZERO;
                held_back = true;
            }
        }
        // across
        let turned_t = pose.t;
        let mut knocked = None;
        let across = v3(per(vel.x), 0, per(vel.z));
        if across != V3::ZERO {
            let cand = pose.at(pose.t.add(across));
            match self.wall(ground, &near, &samples(&pose), &samples(&cand)) {
                Some(hit) => {
                    // a loose tile it runs into, with nothing on it, is
                    // knocked on with some of its speed
                    if let Some(j) = hit {
                        if came.longer_than(mm1(KNOCK_MIN_MM_S)) {
                            knocked = Some((j, came));
                        }
                    }
                    vel = v3(0, vel.y, 0);
                    held_back = true;
                }
                None => pose = cand,
            }
        }
        // down (or up its face), then out of whatever it went into
        pose.t.y += per(vel.y);
        let lift = self.sunk(ground, &near, &old, &samples(&pose));
        if lift > 0 {
            pose.t.y += lift;
            if flying && vel.y < 0 {
                // it lands, with a small hop if it came down hard
                let hit = -vel.y;
                let hop = if hit > mm1(BOUNCE_FROM_MM_S) {
                    hit * BOUNCE_PCT / 100
                } else {
                    0
                };
                vel = v3(vel.x * LAND_KEEP_PCT / 100, hop, vel.z * LAND_KEEP_PCT / 100);
            }
        }
        if !flying {
            // touching, its speed is what it really moved: a slide into
            // the ground or a wall goes nowhere and keeps no speed
            let d = pose.t.sub(turned_t);
            let k = 1000 / dt_ms.max(1);
            vel = v3(d.x * k, d.y * k, d.z * k);
        }
        // turned past upright: the same box the other way up
        let mut flipped = false;
        if pose.c1.y < 0 {
            pose = pose
                .then(&Affine::translate(v3(0, mm1(THICK_MM), 0)))
                .then(&Affine::rot_z(fx::HALF));
            flipped = true;
        }
        let moved = if flipped {
            i32::MAX
        } else {
            old.iter()
                .zip(samples(&pose).iter())
                .map(|(a, b)| (a.x - b.x).abs().max((a.y - b.y).abs()).max((a.z - b.z).abs()))
                .max()
                .unwrap_or(0)
        };
        // its middle is held: over what it touches, or leaning on the side
        // of a tile; a tile tipping over an edge never sleeps there
        let held = tip.is_none() || leans;
        let still = !flying
            && held
            && moved < mm1(1) * STILL_UM / 1000
            && (held_back || (spin == V3::ZERO && !vel.longer_than(mm1(STILL_MM_S))));
        let t = &mut self.all[i];
        t.awake_ticks += 1;
        t.pose = pose;
        t.vel = vel;
        t.spin = spin;
        t.face_down ^= flipped;
        t.quiet = if still { t.quiet.saturating_add(1) } else { 0 };
        let id = t.id;
        // the time limit: a tile still tipping over an edge gets as long
        // again to come off it
        let force = t.awake_ticks >= ph.settle_ticks * if held { 1 } else { 2 };
        // a tile leaning in a pile that only creeps (a little slip, a
        // lift back) sleeps too
        let mut crept = false;
        if t.awake_ticks.is_multiple_of(CREEP_TICKS) {
            let m = t.middle();
            // (one still tipping gets a second for its tip to show: a tip
            // that gets nowhere is held by what the samples miss)
            let tried = held || t.awake_ticks >= 2 * CREEP_TICKS;
            crept = !flying && tried && !m.sub(t.creep_from).longer_than(mm1(CREEP_MM));
            t.creep_from = m;
        }
        let rest = t.quiet >= QUIET_TICKS || force || crept;
        if moved > mm1(1) / 4 {
            self.wake_above(id);
        }
        if let Some((j, v)) = knocked {
            self.knock(j, v.scale(ONE * KNOCK_PCT / 100));
        }
        if !rest {
            return false;
        }
        if force {
            // put down where it is: on what is under it, never in it
            let pts = samples(&pose);
            let gap = pts
                .iter()
                .map(|q| q.y - self.support_near(ground, &near, *q, mm1(CONTACT_MM)))
                .min()
                .unwrap_or(0);
            if gap > 0 {
                self.all[i].pose.t.y -= gap;
            }
        }
        let t = &mut self.all[i];
        t.stop();
        t.state = State::Loose { awake: false };
        let on = self.lying_on(i);
        self.all[i].rests_on = on;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::tiles::tests::{lying, physics};
    use super::super::tiles::{Tile, LEFT};
    use super::*;

    fn run(ts: &mut Tiles, g: &Ground, ticks: u32) -> u32 {
        let ph = physics();
        for k in 0..ticks {
            ts.step(g, &ph, 20);
            if !ts.any_awake() {
                return k + 1;
            }
        }
        ticks
    }

    fn awake(mut t: Tile) -> Tile {
        t.state = State::Loose { awake: true };
        t
    }

    /// The angle (degrees) between a tile's up and straight up.
    fn tilt_deg(t: &Tile) -> i32 {
        let a = fx::atan2(t.pose.c1.flat().len(), t.pose.c1.y);
        a * 360 / fx::TURN
    }

    #[test]
    fn a_tile_dropped_flat_on_the_sand_lands_flat_and_sleeps_on_it() {
        let mut ts = Tiles {
            all: vec![awake(lying(0, [0, 120, 0]))],
        };
        let g = Ground::flat(2000);
        let ticks = run(&mut ts, &g, 400);
        assert!(ticks < 60, "it lands and sleeps soon: {ticks} ticks");
        let t = &ts.all[0];
        assert!(t.pose.t.y.abs() <= mm1(1), "on the sand: {}", t.pose.t.y);
        assert_eq!(tilt_deg(t), 0);
    }

    #[test]
    fn a_tile_landing_with_its_middle_past_a_tiles_edge_tips_off_it_onto_the_sand() {
        // B dropped so that 70 mm of it is over A, its middle 30 mm past
        // A's edge: it tips over that edge and lands on the sand, beyond A
        let mut ts = Tiles {
            all: vec![lying(0, [0, 0, 0]), awake(lying(1, [130, 60, 0]))],
        };
        let g = Ground::flat(2000);
        run(&mut ts, &g, 400);
        assert!(!ts.any_awake());
        let b = &ts.all[1];
        // by hand: it pivots on A's edge (x = 100) and its far side goes
        // down to the sand; it leans on A's edge, its near edge up on A
        // (45 mm), or slides off flat. Either way its middle is past
        // A's edge and lower than where it started, and it turned.
        assert!(b.pose.t.x > mm1(100), "beyond A: {}", b.pose.t.x);
        assert!(b.middle().y < mm1(60), "it came down: {}", b.middle().y);
        assert!(
            tilt_deg(b) <= 27,
            "it rests no steeper than repose: {}",
            tilt_deg(b)
        );
        assert_eq!(
            b.rests_on.contains(&0),
            b.pose.c1.y < ONE * 999 / 1000,
            "{:?} {:?} {}",
            b.rests_on,
            b.pose,
            tilt_deg(b)
        );
    }

    #[test]
    fn a_tile_landing_over_a_tiles_middle_stays_on_it_flat() {
        let mut ts = Tiles {
            all: vec![lying(0, [0, 0, 0]), awake(lying(1, [40, 80, -30]))],
        };
        let g = Ground::flat(2000);
        run(&mut ts, &g, 400);
        let b = &ts.all[1];
        assert!((b.pose.t.y - mm1(45)).abs() <= mm1(1), "{}", b.pose.t.y);
        assert!((b.pose.t.x - mm1(40)).abs() <= mm1(1), "it did not move across");
        assert_eq!(tilt_deg(b), 0);
        assert_eq!(b.rests_on, vec![0]);
    }

    #[test]
    fn a_tile_landing_on_its_edge_falls_onto_a_face() {
        // let go on one edge, tipped up 60, 90 or 115 degrees
        for (deg, ends) in [(60, 0), (90, 90), (115, 180)] {
            let mut t = lying(0, [0, 0, 0]);
            t.pose = t.pose.then(&Affine::rot_z(fx::deg(deg)));
            // its lowest point 5 mm over the sand
            let low = samples(&t.pose).iter().map(|q| q.y).min().unwrap();
            t.pose.t.y += mm1(5) - low;
            let mut ts = Tiles { all: vec![awake(t)] };
            let g = Ground::flat(2000);
            run(&mut ts, &g, 400);
            let t = &ts.all[0];
            // by hand: its middle is 22.5 mm up from its underside and
            // 100 mm in from each edge, so on its 45 mm side it balances
            // between 90 - atan(22.5 / 100) = 77 and 103 degrees. At 60 it
            // falls back onto its underside; at 90 it stands on its side;
            // at 115 it falls onto its top: turned over, flat.
            let tilt = if t.face_down {
                180 - tilt_deg(t)
            } else {
                tilt_deg(t)
            };
            assert!((tilt - ends).abs() <= 2, "{deg}: ends at {ends}, not {tilt}");
            for q in samples(&t.pose) {
                assert!(q.y >= -mm1(1), "{deg}: never in the sand");
            }
            assert_eq!(t.face_down, ends == 180, "{deg}: turned over");
        }
    }

    fn slope(deg: i32) -> Ground {
        // a ramp rising toward +x: height = x tan(deg)
        let mut g = Ground::flat(2000);
        g.tilt_x(deg);
        g
    }

    #[test]
    fn on_a_gentle_slope_a_tile_stays_and_on_a_steep_one_it_slides_down() {
        for (deg, slides) in [(15, false), (40, true)] {
            let g = slope(deg);
            let mut t = lying(0, [0, 300, 0]);
            t.pose = t.pose.then(&Affine::rot_z(fx::deg(deg)));
            let mut ts = Tiles { all: vec![awake(t)] };
            run(&mut ts, &g, 600);
            let t = &ts.all[0];
            // by hand: it lands on the ramp at x = 0; at 15 degrees
            // (under 27) it holds; at 40 it slides more than 200 mm toward
            // -x, down the ramp
            if slides {
                assert!(t.pose.t.x < mm1(-200), "{deg}: it slid down {}", t.pose.t.x);
            } else {
                assert!(t.pose.t.x.abs() < mm1(30), "{deg}: it held {}", t.pose.t.x);
                assert!(
                    (tilt_deg(t) - deg).abs() <= 2,
                    "it lies on the ramp: {}",
                    tilt_deg(t)
                );
            }
            for q in samples(&t.pose) {
                assert!(q.y >= g.height(q.x, q.z) - mm1(1), "never in the ramp");
            }
        }
    }

    #[test]
    fn pulling_one_of_two_tiles_under_a_bridge_tips_it_down_onto_the_sand() {
        // C lies across A and B (a gap between them); B is grabbed: C's
        // middle is over the gap, past A's edge, so it tips down off A
        let mut ts = Tiles {
            all: vec![
                lying(0, [-120, 0, 0]),
                lying(1, [120, 0, 0]),
                lying(2, [0, 45, 0]),
            ],
        };
        ts.find_rests();
        assert_eq!(ts.all[2].rests_on, vec![0, 1]);
        ts.all[1].state = State::Held { seat: 0, claw: LEFT };
        ts.wake_above(1);
        let g = Ground::flat(2000);
        run(&mut ts, &g, 400);
        let c = &ts.all[2];
        // by hand: it turns about A's edge (x = -20) until its far side is
        // on the sand: leaning on A at about 13 degrees (45 mm over about
        // 200), or slid off it flat
        assert!(
            tilt_deg(c) > 5 || c.pose.t.y.abs() < mm1(1),
            "it came down: {}",
            tilt_deg(c)
        );
        assert!(c.middle().y < mm1(45 + 22), "lower than it was");
        for q in c.underside() {
            assert!(q.y >= -mm1(1), "never under the sand");
        }
    }

    #[test]
    fn a_tile_sliding_into_the_side_of_another_stops_against_it_and_knocks_it_on() {
        // B slides at 250 mm/s toward A, 20 mm away; on the sand friction
        // would stop it after about 30 mm (0.25² / 2 / 1.0 m/s²). It meets
        // A 22 mm on (2 mm inside A's side), at about 130 mm/s
        // (√(0.25² - 2 × 1.0 × 0.022)), and stops; A takes 60% of that,
        // about 80 mm/s, and slides at most 0.08² / 2 / 1.0 = 3 mm
        let mut ts = Tiles {
            all: vec![lying(0, [0, 0, 0]), awake(lying(1, [-220, 0, 0]))],
        };
        ts.all[1].vel = v3(mm1(250), 0, 0);
        let g = Ground::flat(2000);
        run(&mut ts, &g, 400);
        let b = &ts.all[1];
        assert!(b.pose.t.x <= mm1(-198), "it stopped at A's side: {}", b.pose.t.x);
        assert!(b.pose.t.x > mm1(-215), "it slid");
        assert!(b.pose.t.y.abs() <= mm1(1), "still on the sand");
        let a = ts.all[0].pose.t.x;
        assert!(a > mm1(1) / 2 && a < mm1(4), "A knocked on a little: {a}");
    }

    /// The shipped tuning's numbers (R16.1), for the tests of how heavy a
    /// tile is; the tests above keep their own, hand-worked numbers.
    fn shipped() -> Physics {
        let t = crate::tuning::parse(crate::tuning::tests::SHIPPED).unwrap();
        crate::sim::Rules::new(&t).physics
    }

    /// How far a tile's lowest point is over what is under it (the sand or
    /// another tile's top), on a 5 mm grid over its underside and sides.
    fn hover_mm(ts: &Tiles, g: &Ground, i: usize) -> i32 {
        let t = &ts.all[i];
        let mut best = i32::MAX;
        for a in (-100..=100).step_by(5) {
            for (u, y, w) in (0..=45)
                .step_by(5)
                .flat_map(|y| [(a, y, -100), (a, y, 100), (-100, y, a), (100, y, a)])
                .chain((-100..=100).step_by(5).map(|b| (a, 0, b)))
            {
                let p = t.pose.apply(v3(mm1(u), mm1(y), mm1(w)));
                let mut s = g.height(p.x, p.z);
                for (j, o) in ts.all.iter().enumerate() {
                    if let Some(h) = o.top_at(p.x, p.z).filter(|h| j != i && *h <= p.y + mm1(1)) {
                        s = s.max(h);
                    }
                }
                best = best.min(p.y - s);
            }
        }
        best * 1000 / ONE
    }

    #[test]
    fn a_tile_let_go_over_nothing_falls_to_the_sand_fast() {
        // let go 300 mm over the sand: by hand, at 9.8 m/s² it lands after
        // √(2 × 0.3 / 9.8) = 0.25 s (13 ticks), at the old 2.5 m/s² after
        // 0.49 s (25 ticks)
        let mut ts = Tiles {
            all: vec![awake(lying(0, [0, 300, 0]))],
        };
        let g = Ground::flat(2000);
        let ph = shipped();
        // the tick it first reaches the sand, and how high it gets after
        let (mut down, mut hop) = (None, 0);
        for k in 0..120 {
            ts.step(&g, &ph, 20);
            let y = ts.all[0].pose.t.y;
            if down.is_some() {
                hop = hop.max(y);
            } else if y.abs() <= mm1(1) {
                down = Some(k + 1);
            }
        }
        let down = down.expect("it lands");
        assert!(down <= 16, "down on the sand by 0.32 s: tick {down}");
        // it came down at about 2.4 m/s (√(2 × 9.8 × 0.3)) and hops back
        // up with 12% of that, 0.29 m/s, less a tick of the sand's grip:
        // at most 0.29² / 2 / 9.8 = 4 mm, then lies still
        assert!(hop > mm1(1) && hop <= mm1(5), "a small hop: {hop}");
        assert!(!ts.any_awake());
        assert!(ts.all[0].pose.t.y.abs() <= mm1(1));
    }

    #[test]
    fn a_tile_resting_only_on_a_corner_of_another_tips_off_it() {
        // B lies flat with only a 30 × 30 mm corner of it over A's corner
        // (its middle 170 mm past A's middle each way, 240 mm off): its
        // middle is far past what holds it, so it tips off A's corner and
        // comes down onto the sand, leaning on A's corner at most
        let mut ts = Tiles {
            all: vec![lying(0, [0, 0, 0]), awake(lying(1, [170, 45, 170]))],
        };
        let g = Ground::flat(2000);
        let ph = shipped();
        for _ in 0..400 {
            ts.step(&g, &ph, 20);
            if !ts.any_awake() {
                break;
            }
        }
        assert!(!ts.any_awake());
        let b = &ts.all[1];
        // by hand: lying flat on A it would have its underside at 45 mm and
        // its middle 67 mm up; come down, its far side is on the sand, so
        // its middle is at most half way up A's 45 mm plus its own half
        // (22 + 22 = 45 mm)
        assert!(b.middle().y <= mm1(46), "it came down off A: {}", b.middle().y);
        let low = samples(&b.pose).iter().map(|q| q.y).min().unwrap();
        assert!(low.abs() <= mm1(2), "a side of it is on the sand: {low}");
        assert!(hover_mm(&ts, &g, 1) <= 2, "it hovers {} mm", hover_mm(&ts, &g, 1));
    }

    #[test]
    fn the_time_limit_never_leaves_a_tile_balanced_on_an_edge() {
        // B lies flat on A with its middle 30 mm past A's edge, starting
        // to tip, one tick short of the time limit: put down where it is
        // it would stay on A's edge with nothing under its middle; instead
        // it gets longer and tips off, its far side down on the sand
        let mut b = awake(lying(1, [130, 45, 0]));
        let ph = shipped();
        b.awake_ticks = ph.settle_ticks - 1;
        let mut ts = Tiles {
            all: vec![lying(0, [0, 0, 0]), b],
        };
        let g = Ground::flat(2000);
        for _ in 0..400 {
            ts.step(&g, &ph, 20);
            if !ts.any_awake() {
                break;
            }
        }
        let b = &ts.all[1];
        // by hand: flat on A its middle is 67 mm up. Turned about A's edge
        // (x = 100, 45 mm up) until its far edge (x = 230) is on the sand,
        // its underside falls 45 mm in 130 (19 degrees, under repose, so it
        // leans there): its middle, 30 mm past the edge, is 45 - 30 × 45 /
        // 130 = 35 mm up its underside and 22.5 × cos 19° = 21 mm over it,
        // 56 mm; lower if it slid off
        assert!(b.middle().y <= mm1(57), "it tipped off A: {}", b.middle().y);
        let low = samples(&b.pose).iter().map(|q| q.y).min().unwrap();
        assert!(low.abs() <= mm1(2), "its far side is on the sand: {low}");
        assert!(hover_mm(&ts, &g, 1) <= 2, "it hovers {} mm", hover_mm(&ts, &g, 1));
    }

    #[test]
    fn the_same_tumble_twice_comes_out_the_same() {
        let make = || {
            let mut t = lying(3, [170, 160, 40]);
            t.pose = t
                .pose
                .then(&Affine::rot_x(fx::deg(25)))
                .then(&Affine::rot_y(fx::deg(30)));
            Tiles {
                all: vec![
                    lying(0, [0, 0, 0]),
                    lying(1, [10, 45, 20]),
                    lying(2, [-150, 0, 90]),
                    awake(t),
                ],
            }
        };
        let g = Ground::flat(2000);
        let (mut a, mut b) = (make(), make());
        a.find_rests();
        b.find_rests();
        run(&mut a, &g, 400);
        run(&mut b, &g, 400);
        assert_eq!(a, b);
        assert!(!a.any_awake());
    }
}
