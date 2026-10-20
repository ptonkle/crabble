//! The legs (TRIAGE F1, "the legs being able to dynamically place
//! themselves"): each foot stands on what is under it (the sand, a mound,
//! the board, a tile, a pile), and steps when the crab has carried it too
//! far from its place under the body; the knee bends to reach; the body's
//! height and tilt follow the feet.
//!
//! - **Places.** A leg's place is its tip at rest, under the crab as it
//!   now stands and faces, on the ground there: the highest of the drawn
//!   ground and any tile at rest (`Match`'s own heights, so a foot stands
//!   where a crab would).
//! - **Steps.** A planted foot stays where it is in the world. When it is
//!   more than `step_mm` from its place it steps: lifted `lift_mm` over the
//!   higher of its two ends and set down over `step_ms` a little ahead of
//!   its place (`lead_ms` of the crab's speed), the place followed as the
//!   crab goes on. The legs step in two alternating fours, as a real
//!   crab's (left 1, right 2, left 3, right 4, then the others): a four
//!   steps only while none of the other is in the air. A foot stretched
//!   past three times `step_mm` steps whatever the other four is doing, and one
//!   left far behind (a respawn, a push through a pile) is set straight in
//!   its place. Standing still, a foot more than `settle_mm` out steps home.
//! - **Reach.** Each leg is two links in its own upright plane, hip to
//!   knee and knee to tip (the crab model's legs are drawn that way): the
//!   hip swings the plane round to the foot, then hip and knee bend, knee
//!   up, to put the tip on it.
//! - **Body.** The body rises or sinks toward the feet's mean height
//!   (`follow_pct`, at most `body_mm`) and tilts to the line from the left
//!   feet to the right and the front feet to the back (at most
//!   `tilt_deg`).
//! - **Air.** In the air the keyed jump moves the legs; the feet hand over
//!   to it over `blend_ms` as the crab leaves the ground and take back over
//!   as it lands, every foot then planted in its place.
//!
//! Everything comes from the crab's state and the ground, so it is the same
//! for a crab another browser moves, and costs a few dozen height lookups
//! a crab a frame.

use serde::Deserialize;

use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::{v3, Affine, V3};
use crate::model::{mm1, Model};

/// The legs' numbers (`data/look.json` `crab.legs`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegsLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub step_mm: i32,
    pub settle_mm: i32,
    pub lift_mm: i32,
    pub step_ms: i32,
    pub lead_ms: i32,
    pub blend_ms: i32,
    pub follow_pct: i32,
    pub body_mm: i32,
    pub tilt_deg: i32,
}

impl LegsLook {
    pub fn check(&self) -> bool {
        (1..=200).contains(&self.step_mm)
            && (0..self.step_mm).contains(&self.settle_mm)
            && (0..=100).contains(&self.lift_mm)
            && (10..=2000).contains(&self.step_ms)
            && (0..=2000).contains(&self.lead_ms)
            && (1..=2000).contains(&self.blend_ms)
            && (0..=100).contains(&self.follow_pct)
            && (0..=100).contains(&self.body_mm)
            && (0..=45).contains(&self.tilt_deg)
    }
}

/// One walking leg of the model: its three parts and their rest points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leg {
    pub hip: usize,
    pub knee: usize,
    pub foot: usize,
    /// the hip and knee pivots and the tip at rest, in the model
    pub h: V3,
    pub k: V3,
    pub tip: V3,
    /// which four of the walk it steps with (0 or 1)
    pub group: usize,
    pub left: bool,
    /// the front two pairs (for the body's pitch)
    pub front: bool,
}

/// The model's eight walking legs (`leg_l1_hip` ... `leg_r4_foot`), or
/// none when it lacks any of them.
pub fn legs_of(m: &Model) -> Vec<Leg> {
    let find = |name: String| m.parts.iter().position(|p| p.name == name);
    let mut out = Vec::new();
    for (left, s) in [(true, "l"), (false, "r")] {
        for n in 1..=4 {
            let (Some(hip), Some(knee), Some(foot)) = (
                find(format!("leg_{s}{n}_hip")),
                find(format!("leg_{s}{n}_knee")),
                find(format!("leg_{s}{n}_foot")),
            ) else {
                return Vec::new();
            };
            // the tip: the foot's lowest point
            let Some(tip) = m
                .verts
                .iter()
                .filter(|(_, p)| *p == foot)
                .map(|(v, _)| *v)
                .min_by_key(|v| v.y)
            else {
                return Vec::new();
            };
            // left 1 and 3, right 2 and 4 step together
            let group = usize::from((n % 2 == 1) != left);
            out.push(Leg {
                hip,
                knee,
                foot,
                h: m.parts[hip].pivot,
                k: m.parts[knee].pivot,
                tip,
                group,
                left,
                front: n <= 2,
            });
        }
    }
    out
}

/// A turn by `a` about the unit axis `n` (right-handed).
pub fn rot_axis(n: V3, a: i32) -> Affine {
    let (s, c) = (fx::sin(a), fx::cos(a));
    let col = |e: V3| {
        let along = n.scale(fx::mul(n.dot(e), ONE - c));
        e.scale(c).add(n.cross(e).scale(s)).add(along)
    };
    Affine {
        c0: col(v3(ONE, 0, 0)),
        c1: col(v3(0, ONE, 0)),
        c2: col(v3(0, 0, ONE)),
        t: V3::ZERO,
    }
}

/// The inverse of a turn and a move (no scale).
pub fn rigid_inverse(a: &Affine) -> Affine {
    let r = Affine {
        c0: v3(a.c0.x, a.c1.x, a.c2.x),
        c1: v3(a.c0.y, a.c1.y, a.c2.y),
        c2: v3(a.c0.z, a.c1.z, a.c2.z),
        t: V3::ZERO,
    };
    Affine {
        t: r.apply_dir(a.t).neg(),
        ..r
    }
}

fn hypot(a: i32, b: i32) -> i32 {
    fx::isqrt((a as i128 * a as i128 + b as i128 * b as i128) as u128) as i32
}

/// About `p`: `t` there.
fn about(p: V3, t: &Affine) -> Affine {
    Affine::translate(p).then(t).then(&Affine::translate(p.neg()))
}

/// The transforms of leg `l`'s hip, knee and foot parts under the body's
/// transform `body` (a turn and a move), putting its tip at `target` (in
/// the model's frame), or as near as the leg reaches.
pub fn solve(l: &Leg, body: &Affine, target: V3) -> [Affine; 3] {
    let t = rigid_inverse(body).apply(target);
    let up = V3::UP;
    let d0 = l.tip.sub(l.h).flat().norm();
    let rel = t.sub(l.h);
    let flat = rel.flat();
    let d1 = if flat.len() > 0 { flat.norm() } else { d0 };
    // swing the leg's plane round to the target
    let swing = fx::atan2(up.dot(d0.cross(d1)), d0.dot(d1));
    // in the plane: out along d0 and up
    let plane = |p: V3| (p.sub(l.h).dot(d0), p.y - l.h.y);
    let (rk, yk) = plane(l.k);
    let (rt, yt) = plane(l.tip);
    let l1 = hypot(rk, yk) as i64;
    let l2 = hypot(rt - rk, yt - yk) as i64;
    let (r, y) = (flat.len(), rel.y);
    let reach = hypot(r, y) as i64;
    let dist = reach.clamp((l1 - l2).abs() + 1, l1 + l2 - 1).max(1);
    let ang = fx::atan2(y, r);
    let cos = (((l1 * l1 + dist * dist - l2 * l2) << 16) / (2 * l1 * dist).max(1))
        .clamp(-ONE as i64, ONE as i64) as i32;
    let open = fx::atan2(fx::sqrt(ONE - fx::mul(cos, cos)), cos);
    // knee up
    let a1 = ang + open;
    let (kx, ky) = (fx::mul(l1 as i32, fx::cos(a1)), fx::mul(l1 as i32, fx::sin(a1)));
    let (px, py) = (
        fx::mul(dist as i32, fx::cos(ang)),
        fx::mul(dist as i32, fx::sin(ang)),
    );
    let a2 = fx::atan2(py - ky, px - kx);
    let lift = a1 - fx::atan2(yk, rk);
    let bend = a2 - fx::atan2(yt - yk, rt - rk) - lift;
    // a turn about n0 by + lifts the leg's end
    let n0 = d0.cross(up).norm();
    let hip = body.then(&about(l.h, &rot_axis(up, swing).then(&rot_axis(n0, lift))));
    let knee = hip.then(&about(l.k, &rot_axis(n0, bend)));
    [hip, knee, knee]
}

/// One foot: planted where it stands in the world, or stepping from
/// where it left, since when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Foot {
    pub at: V3,
    pub step: Option<(V3, i64)>,
}

/// What the feet did this frame: where each tip goes (in the world), how
/// much they lead the pose (Q16, 0 in the air: the keyed jump), and the
/// body's lift (Q16 m) and roll and pitch (angle units).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stance {
    pub tips: Vec<V3>,
    pub weight: i32,
    pub lift: i32,
    pub roll: i32,
    pub pitch: i32,
}

/// The feet a rig keeps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Feet {
    pub feet: Vec<Foot>,
    /// when the crab last landed or left the ground, and which
    pub change: Option<(i64, bool)>,
    pub air: bool,
}

/// The crab this frame, as the feet need it.
pub struct Body {
    /// its place in the world: where it stands, facing (no stretch)
    pub place: Affine,
    /// its speed across the ground (Q16 m/s)
    pub vel: V3,
    pub on_ground: bool,
    pub moving: bool,
    pub now: i64,
}

fn ease(t: i32) -> i32 {
    let t2 = fx::mul(t, t);
    3 * t2 - 2 * fx::mul(t2, t)
}

impl Feet {
    /// Moves the feet on to `now` over `ground` (the height at x, z) and
    /// says where they are.
    pub fn update(
        &mut self,
        legs: &[Leg],
        b: &Body,
        ground: &dyn Fn(i32, i32) -> i32,
        look: &LegsLook,
    ) -> Stance {
        let now = b.now;
        let on = |p: V3| v3(p.x, ground(p.x, p.z), p.z);
        let homes: Vec<V3> = legs.iter().map(|l| on(b.place.apply(l.tip))).collect();
        let lead = b.vel.flat().scale(look.lead_ms * ONE / 1000);
        let step = mm1(look.step_mm);
        if self.feet.len() != legs.len() {
            self.feet = homes.iter().map(|&at| Foot { at, step: None }).collect();
            self.air = !b.on_ground;
        }
        if self.air != !b.on_ground {
            self.air = !b.on_ground;
            self.change = Some((now, b.on_ground));
            if b.on_ground {
                // down: every foot in its place
                for (f, &h) in self.feet.iter_mut().zip(&homes) {
                    *f = Foot { at: h, step: None };
                }
            }
        }
        // steps that are done land
        for f in &mut self.feet {
            if let Some((_, since)) = f.step {
                if now - since >= look.step_ms as i64 {
                    f.step = None;
                }
            }
        }
        if b.on_ground {
            for j in 0..legs.len() {
                let f = self.feet[j];
                let target = on(homes[j].add(lead));
                if f.step.is_some() {
                    self.feet[j].at = target;
                    continue;
                }
                // a planted foot stays on what is under it
                let at = on(f.at);
                self.feet[j].at = at;
                let off = at.sub(homes[j]).flat().len();
                let other_up = legs
                    .iter()
                    .zip(&self.feet)
                    .any(|(l, g)| l.group != legs[j].group && g.step.is_some());
                let limit = if b.moving { step } else { mm1(look.settle_mm) };
                if off > 6 * step {
                    self.feet[j].at = homes[j];
                } else if (off > limit && !other_up) || off > 3 * step {
                    self.feet[j] = Foot {
                        at: target,
                        step: Some((at, now)),
                    };
                }
            }
        }
        // where each tip is: planted, or on its arc
        let lift = mm1(look.lift_mm);
        let mut ground_line = Vec::with_capacity(legs.len());
        let tips = self
            .feet
            .iter()
            .map(|f| match f.step {
                None => {
                    ground_line.push(f.at);
                    f.at
                }
                Some((from, since)) => {
                    let t = fx::ratio((now - since).clamp(0, look.step_ms as i64) as i32, look.step_ms);
                    let s = ease(t);
                    let mut p = from.lerp(f.at, s);
                    ground_line.push(p);
                    // over the higher end: a step up a tile clears its edge
                    p.y += fx::mul(lift + (f.at.y - from.y).abs() / 2, fx::sin(t / 2));
                    p
                }
            })
            .collect();
        // the hand-over between the feet and the keyed jump
        let blend = |ms: i64| fx::ratio(ms.clamp(0, look.blend_ms as i64) as i32, look.blend_ms);
        let weight = match (self.change, b.on_ground) {
            (Some((at, true)), true) => blend(now - at),
            (Some((at, false)), false) => ONE - blend(now - at),
            (None, true) => ONE,
            _ => 0,
        };
        // the body follows the feet: in the crab's own frame
        let inv = rigid_inverse(&b.place);
        let local: Vec<V3> = ground_line.iter().map(|p| inv.apply(*p)).collect();
        let mean = |pick: &dyn Fn(&Leg) -> bool| {
            let pts: Vec<V3> = legs
                .iter()
                .zip(&local)
                .filter(|(l, _)| pick(l))
                .map(|(_, p)| *p)
                .collect();
            let n = pts.len().max(1) as i32;
            pts.iter().fold(V3::ZERO, |a, p| a.add(*p)).scale(ONE / n)
        };
        let all = mean(&|_| true);
        let (l, r) = (mean(&|g| g.left), mean(&|g| !g.left));
        let (fr, bk) = (mean(&|g| g.front), mean(&|g| !g.front));
        let most = deg(look.tilt_deg);
        let body_mm = mm1(look.body_mm);
        Stance {
            tips,
            weight,
            lift: (all.y as i64 * look.follow_pct as i64 / 100) as i32,
            roll: fx::atan2(l.y - r.y, (l.x - r.x).max(1)).clamp(-most, most),
            pitch: (-fx::atan2(fr.y - bk.y, (fr.z - bk.z).max(1))).clamp(-most, most),
        }
        .clamped(body_mm)
    }
}

impl Stance {
    fn clamped(mut self, mm: i32) -> Stance {
        self.lift = self.lift.clamp(-mm, mm);
        self
    }
}

#[cfg(test)]
mod tests;
