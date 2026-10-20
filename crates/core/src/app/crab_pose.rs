//! A crab's pose past its keyed moves (TRIAGE F12, "make crab animation
//! more dynamic"): drawn from what the core already knows of each crab (its
//! gait, speed, turn, time in the air, last grab or place), the same for a
//! crab this browser moves and one another browser does. Nothing here
//! changes a rule; the numbers are `data/look.json` `crab`.
//!
//! On top of the model's walk, jump, hold and reach moves (`model.rs`):
//!
//! - the body bobs with each step, leans into a scuttle or a shuffle and
//!   banks into a turn, the lean easing in and out over `lean_ms`;
//! - the whole crab stretches as it leaves the ground and squashes as it
//!   lands (harder the faster it came down), with a small rebound;
//! - the eye stalks trail a move or a turn and nod on landing;
//! - the claws sway as it walks; a claw squeezes and jerks as it grabs a
//!   tile, and clacks twice after it places one;
//! - a crab that stands still a while fidgets now and then: it looks
//!   about, clacks its empty claws or shifts its weight. Which, and when,
//!   comes from its seat and the clock, so every browser shows the same.
//!
//! [`Rig`] keeps what a pose needs from earlier frames (the turn rate, the
//! eased lean, when it landed, where each foot stands); `View3d` holds one
//! per crab. The legs (TRIAGE F1) are `legs.rs`.

use serde::Deserialize;

use crate::gfx::fx::{self, deg, ONE, TURN};
use crate::gfx::geom::{facing, v3, Affine, V3};
use crate::model::{mm1, Model, Pose};
use crate::rng::mix;
use crate::sim::{Crab, Gait, Reach};

pub mod legs;

/// The numbers of a crab's extra motion (`data/look.json` `crab`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrabLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// how far the body dips with each step, mm
    pub bob_mm: i32,
    /// the body's roll into a sideways scuttle, and its pitch into a walk
    /// forward or back, at full speed, degrees
    pub lean_deg: i32,
    pub pitch_deg: i32,
    /// the body's bank into a turn of half a turn a second, degrees
    pub turn_lean_deg: i32,
    /// how quickly the lean and the turn follow (a time constant, ms)
    pub lean_ms: i32,
    /// how much taller the crab is as it leaves the ground (percent), easing
    /// back over `stretch_ms`
    pub stretch_pct: i32,
    pub stretch_ms: i32,
    /// how much shorter it is as it lands from a full jump (percent), with a
    /// rebound, over `squash_ms`
    pub squash_pct: i32,
    pub squash_ms: i32,
    /// how far the eye stalks trail a move or a turn at full speed, and
    /// nod on landing, degrees
    pub eye_lag_deg: i32,
    pub eye_bounce_deg: i32,
    /// the claws' sway as it walks, degrees
    pub claw_sway_deg: i32,
    /// a claw's squeeze on a grab and clack after a place, degrees, and
    /// how long each takes
    pub snap_deg: i32,
    pub snap_ms: i32,
    /// standing still this long, it fidgets: one every `fidget_every_ms`
    /// (a little more for later seats), each `fidget_ms` long and up to
    /// `fidget_deg`
    pub fidget_after_ms: i32,
    pub fidget_every_ms: i32,
    pub fidget_ms: i32,
    pub fidget_deg: i32,
    /// the legs: where the feet stand and how they step (`legs.rs`)
    pub legs: legs::LegsLook,
}

impl CrabLook {
    pub fn check(&self) -> Result<(), String> {
        let ok = (0..=20).contains(&self.bob_mm)
            && [
                self.lean_deg,
                self.pitch_deg,
                self.turn_lean_deg,
                self.eye_lag_deg,
                self.eye_bounce_deg,
                self.claw_sway_deg,
                self.snap_deg,
                self.fidget_deg,
            ]
            .iter()
            .all(|d| (0..=45).contains(d))
            && (0..=60).contains(&self.stretch_pct)
            && (0..=60).contains(&self.squash_pct)
            && [
                self.lean_ms,
                self.stretch_ms,
                self.squash_ms,
                self.snap_ms,
                self.fidget_ms,
            ]
            .iter()
            .all(|ms| (1..=5000).contains(ms))
            && self.legs.check()
            && (0..=60_000).contains(&self.fidget_after_ms)
            && (self.fidget_ms..=60_000).contains(&self.fidget_every_ms);
        if ok {
            Ok(())
        } else {
            Err("The crab's motion numbers are out of range.".into())
        }
    }
}

/// The crab model's parts this file moves, by index (left first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parts {
    pub body: usize,
    pub eye: [Option<usize>; 2],
    pub arm: [Option<usize>; 2],
    pub finger: [Option<usize>; 2],
}

impl Parts {
    pub fn of(m: &Model) -> Parts {
        let find = |name: &str| m.parts.iter().position(|p| p.name == name);
        Parts {
            body: find("body").unwrap_or(0),
            eye: [find("eye_l"), find("eye_r")],
            arm: [find("claw_l_arm"), find("claw_r_arm")],
            finger: [find("claw_l_finger"), find("claw_r_finger")],
        }
    }
}

/// What a crab's pose remembers from earlier frames.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rig {
    /// the clock at the last frame (none before the first)
    ms: Option<i64>,
    yaw: i32,
    /// the turn rate, eased (angle units a second; + to its left)
    turn: i32,
    /// the body's roll and pitch, eased (angle units)
    lean: [i32; 2],
    air: bool,
    /// its speed up (Q16 metres a second) at the last frame in the air
    fall: i32,
    /// when it last landed and how hard (Q16, a full jump's fall ONE)
    land: Option<(i64, i32)>,
    /// the clock when it was last moving or in the air
    moved: i64,
    /// the feet (`legs.rs`)
    pub feet: legs::Feet,
}

/// What the rest of the frame needs to know of a crab's game.
#[derive(Debug, Clone, Copy)]
pub struct Context {
    /// the game's clock, ms
    pub now: i64,
    /// the crab's seat
    pub seat: usize,
    /// its walking speed and its jump's take-off speed (Q16 m/s)
    pub speed: i32,
    pub jump_speed: i32,
    /// the walk's loop for its gait, ms
    pub cycle_ms: u32,
}

/// `x` of `of`, as a Q16 fraction from -ONE to ONE.
fn frac(x: i32, of: i32) -> i32 {
    fx::div(x, of.max(1)).clamp(-ONE, ONE)
}

/// `d` degrees times a Q16 fraction, in angle units.
fn tilt(d: i32, f: i32) -> i32 {
    fx::mul(deg(d), f)
}

impl Rig {
    /// Takes in this frame's crab (`yaw`, the way it is drawn facing).
    pub fn update(&mut self, c: &Crab, yaw: i32, cx: &Context, look: &CrabLook) {
        let now = cx.now;
        let Some(last) = self.ms else {
            *self = Rig {
                ms: Some(now),
                yaw,
                air: !c.on_ground,
                moved: now,
                ..Rig::default()
            };
            return;
        };
        let dt = (now - last).clamp(0, 200) as i32;
        self.ms = Some(now);
        if dt > 0 {
            let raw = (fx::wrap(yaw - self.yaw) as i64 * 1000 / dt as i64) as i32;
            let k = fx::ratio(dt, dt + look.lean_ms);
            self.turn = fx::lerp(self.turn, raw.clamp(-2 * TURN, 2 * TURN), k);
            let target = self.lean_target(c, yaw, cx.speed, look);
            for (l, t) in self.lean.iter_mut().zip(target) {
                *l = fx::lerp(*l, t, k);
            }
        }
        self.yaw = yaw;
        if !c.on_ground {
            self.fall = c.vel.y;
        } else if self.air {
            self.land = Some((now, frac(-self.fall, cx.jump_speed).max(0)));
        }
        self.air = !c.on_ground;
        if c.gait != Gait::Still || !c.on_ground {
            self.moved = now;
        }
    }

    /// The crab's speed across and along itself (Q16 fractions of its
    /// walking speed; + to its left, + forward).
    fn local_speed(c: &Crab, yaw: i32, speed: i32) -> (i32, i32) {
        let f = facing(yaw, 0);
        let left = v3(f.z, 0, -f.x);
        let v = c.vel.flat();
        (frac(v.dot(left), speed), frac(v.dot(f), speed))
    }

    /// The roll and pitch the body leans to: its top toward where it goes
    /// and into a turn.
    fn lean_target(&self, c: &Crab, yaw: i32, speed: i32, look: &CrabLook) -> [i32; 2] {
        if !c.on_ground {
            return [0, 0];
        }
        let (side, fwd) = Rig::local_speed(c, yaw, speed);
        lean_to(side, fwd, frac(self.turn, fx::HALF), look)
    }

    /// What the extra motion reads of crab `c` this frame.
    pub fn motion(&self, c: &Crab, yaw: i32, cx: &Context) -> Motion {
        let (side, fwd) = Rig::local_speed(c, yaw, cx.speed);
        Motion {
            seat: cx.seat,
            on_ground: c.on_ground,
            walking: c.on_ground && c.gait != Gait::Still,
            side,
            fwd,
            moving: frac(c.vel.flat().len(), cx.speed),
            turn: frac(self.turn, fx::HALF),
            lean: self.lean,
            phase: (c.walk_ms as i64 * TURN as i64 / cx.cycle_ms.max(1) as i64) as i32,
            air_ms: (!c.on_ground).then_some(c.air_ms),
            land: self.land.map(|(at, hard)| (cx.now - at, hard)),
            still_ms: if c.on_ground && c.gait == Gait::Still {
                cx.now - self.moved
            } else {
                0
            },
        }
    }

    /// [`dress`] for crab `c` as this rig last saw it.
    pub fn dress(
        &self,
        c: &Crab,
        yaw: i32,
        poses: &mut [Pose],
        parts: &Parts,
        cx: &Context,
        look: &CrabLook,
    ) -> V3 {
        let holds = [c.claws[0].is_some(), c.claws[1].is_some()];
        let reach = c.reach.map(|r| r.map(|(k, at)| (k, (cx.now - at).max(0) as u32)));
        dress(&self.motion(c, yaw, cx), holds, reach, poses, parts, look)
    }
}

/// The roll and pitch for a crab going `side` and `fwd` (fractions of its
/// speed) and turning `turn` (of half a turn a second).
pub fn lean_to(side: i32, fwd: i32, turn: i32, look: &CrabLook) -> [i32; 2] {
    // rot_z: + tips the top toward -x (its right); rot_x: + toward +z
    let roll = -tilt(look.lean_deg, side) - tilt(look.turn_lean_deg, turn);
    let pitch = tilt(look.pitch_deg, fwd);
    [roll, pitch]
}

/// What the extra motion reads of a crab, from a [`Rig`] in the game or
/// worked out from the clock on the Title (`app/title_life.rs`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Motion {
    pub seat: usize,
    pub on_ground: bool,
    /// on the ground with a gait
    pub walking: bool,
    /// speed across (+ to its left) and along it, and over the ground
    /// (Q16 fractions of the walking speed)
    pub side: i32,
    pub fwd: i32,
    pub moving: i32,
    /// its turn, a Q16 fraction of half a turn a second (+ to its left)
    pub turn: i32,
    /// the body's roll and pitch (angle units)
    pub lean: [i32; 2],
    /// how far through its walk's loop (angle units, a loop a turn)
    pub phase: i32,
    /// in the air, for how long (ms)
    pub air_ms: Option<u32>,
    /// how long ago it landed and how hard (Q16, a full jump's fall ONE)
    pub land: Option<(i64, i32)>,
    /// how long it has stood still (ms; 0 moving or in the air)
    pub still_ms: i64,
}

/// Adds the extra motion of a crab doing `m` to `poses` (its keyed moves):
/// `holds` says which claws hold a tile and `reach` each claw's last grab
/// or place, how long ago. Returns the whole crab's scale (across, up,
/// across; Q16) for its stretch or squash.
pub fn dress(
    m: &Motion,
    holds: [bool; 2],
    reach: [Option<(Reach, u32)>; 2],
    poses: &mut [Pose],
    parts: &Parts,
    look: &CrabLook,
) -> V3 {
    let phase = m.phase;

    // the body: a dip with each step, the lean
    let body = &mut poses[parts.body];
    if m.walking {
        body.shift.y -= fx::mul(mm1(look.bob_mm), fx::sin(phase).abs());
    }
    body.turn[2] += m.lean[0];
    body.turn[0] += m.lean[1];

    // the eyes trail the move and the turn, and nod on landing
    let nod = m
        .land
        .map(|(since, hard)| {
            let u = fx::ratio(
                since.clamp(0, 2 * look.squash_ms as i64) as i32,
                2 * look.squash_ms,
            );
            fx::mul(
                tilt(look.eye_bounce_deg, hard),
                fx::mul(ONE - u, fx::sin(u * 3 / 2)),
            )
        })
        .unwrap_or(0);
    for e in parts.eye.into_iter().flatten() {
        let p = &mut poses[e];
        p.turn[2] += tilt(look.eye_lag_deg, m.side);
        p.turn[0] += -tilt(look.eye_lag_deg, m.fwd) + nod;
        p.turn[1] -= tilt(look.eye_lag_deg, m.turn);
    }

    // the claws sway with the walk; a grab squeezes, a place clacks
    for k in 0..2 {
        if let Some(a) = parts.arm[k] {
            let sway = fx::mul(tilt(look.claw_sway_deg, m.moving), fx::sin(phase));
            let sway = if holds[k] { sway / 2 } else { sway };
            // both arms swing the same way about their own y
            poses[a].turn[1] += sway;
            poses[a].turn[0] += fx::mul(tilt(look.claw_sway_deg, m.moving), fx::sin(2 * phase)) / 2;
        }
        if let Some((r, since)) = reach[k] {
            let since = since as i64;
            let snap = look.snap_ms as i64;
            match r {
                // after the claw closes on it (the grab's 220 ms key)
                Reach::Grab if (220..220 + snap).contains(&since) => {
                    let u = fx::ratio((since - 220) as i32, look.snap_ms);
                    let s = tilt(look.snap_deg, fx::sin(u / 2));
                    if let Some(f) = parts.finger[k] {
                        poses[f].turn[0] += s / 2;
                    }
                    if let Some(a) = parts.arm[k] {
                        poses[a].turn[0] -= s / 2;
                    }
                }
                // after the place's 440 ms: two clacks
                Reach::Place if (440..440 + 2 * snap).contains(&since) => {
                    let u = fx::ratio((since - 440) as i32, 2 * look.snap_ms);
                    if let Some(f) = parts.finger[k] {
                        poses[f].turn[0] -= tilt(look.snap_deg, fx::sin(u).abs());
                    }
                }
                _ => {}
            }
        }
    }

    // a crab standing still a while fidgets now and then
    fidget(m, holds, poses, parts, look);

    // stretch leaving the ground, squash landing
    let mut up = 0;
    match (m.air_ms, m.land) {
        (Some(air), _) if (air as i32) < look.stretch_ms => {
            let u = fx::ratio(air as i32, look.stretch_ms);
            up = fx::mul(look.stretch_pct * ONE / 100, ONE - u);
        }
        (None, Some((since, hard))) if (0..look.squash_ms as i64).contains(&since) => {
            let u = fx::ratio(since as i32, look.squash_ms);
            // down at once, back past even a little, and still
            let wave = fx::mul(ONE - u, fx::cos(u * 3 / 4));
            up = -fx::mul(fx::mul(look.squash_pct * ONE / 100, hard), wave);
        }
        _ => {}
    }
    v3(ONE - up / 2, ONE + up, ONE - up / 2)
}

fn fidget(m: &Motion, holds: [bool; 2], poses: &mut [Pose], parts: &Parts, look: &CrabLook) {
    let still = m.still_ms - look.fidget_after_ms as i64;
    if m.still_ms <= 0 || still < 0 {
        return;
    }
    let seat = m.seat as u64;
    let every = look.fidget_every_ms as i64 + (mix(seat) % 700) as i64;
    let (round, within) = (still / every, still % every);
    if within >= look.fidget_ms as i64 {
        return;
    }
    let u = fx::ratio(within as i32, look.fidget_ms);
    // in and out over the fidget
    let swell = fx::sin(u / 2);
    let d = deg(look.fidget_deg);
    match mix(seat * 977 + round as u64) % 3 {
        // looks about: both stalks one way, then the other
        0 => {
            for e in parts.eye.into_iter().flatten() {
                poses[e].turn[1] += fx::mul(fx::mul(d, swell), fx::sin(u));
            }
        }
        // clacks its empty claws, three times
        1 => {
            for (finger, held) in parts.finger.into_iter().zip(holds) {
                if let (Some(f), false) = (finger, held) {
                    poses[f].turn[0] -= fx::mul(fx::mul(d, swell), fx::sin(u * 3 / 2).abs());
                }
            }
        }
        // shifts its weight from side to side
        _ => {
            let b = &mut poses[parts.body];
            b.turn[2] += fx::mul(d / 3, fx::mul(swell, fx::sin(u)));
            b.shift.x += fx::mul(mm1(2), fx::mul(swell, fx::sin(u)));
        }
    }
}

/// The extra motion for [`super::view3d::pose_crab`]: what to read, and
/// its numbers.
pub struct Extra<'a> {
    pub motion: Motion,
    pub look: &'a CrabLook,
    /// where the feet stand (none: the keyed legs)
    pub legs: Option<Legs>,
}

/// The legs for [`super::view3d::pose_crab`]: the model's legs, the
/// stance, and the world to the model's frame (the crab's place, not
/// stretched).
pub struct Legs {
    pub legs: Vec<legs::Leg>,
    pub stance: legs::Stance,
    pub to_model: Affine,
}

impl Legs {
    /// Raises and tilts the body to the feet (`poses[body]`).
    pub fn lift_body(&self, poses: &mut [Pose], body: usize) {
        let s = &self.stance;
        let p = &mut poses[body];
        p.shift.y += fx::mul(s.lift, s.weight);
        p.turn[2] += fx::mul(s.roll, s.weight);
        p.turn[0] += fx::mul(s.pitch, s.weight);
    }

    /// Puts each leg's tip where the stance says (blended with the keyed
    /// pose by its weight), over `transforms` (the posed model's, before
    /// the whole crab is scaled by `squash`).
    pub fn reach(&self, transforms: &mut [Affine], body: usize, squash: V3) {
        let s = &self.stance;
        if s.weight <= 0 {
            return;
        }
        let unsquash = Affine::scale(v3(
            fx::div(ONE, squash.x),
            fx::div(ONE, squash.y),
            fx::div(ONE, squash.z),
        ));
        let to = unsquash.then(&self.to_model);
        let b = transforms[body];
        for (l, tip) in self.legs.iter().zip(&s.tips) {
            let keyed = transforms[l.foot].apply(l.tip);
            let target = keyed.lerp(to.apply(*tip), s.weight);
            let [h, k, f] = legs::solve(l, &b, target);
            transforms[l.hip] = h;
            transforms[l.knee] = k;
            transforms[l.foot] = f;
        }
    }
}

impl Rig {
    /// Moves crab `c`'s feet on to `now` over `ground` (the height at x,
    /// z), drawn facing `yaw`. None for a model without the eight legs.
    pub fn stand(
        &mut self,
        m: &Model,
        c: &Crab,
        yaw: i32,
        now: i64,
        ground: &dyn Fn(i32, i32) -> i32,
        look: &CrabLook,
    ) -> Option<Legs> {
        let legs = legs::legs_of(m);
        if legs.is_empty() {
            return None;
        }
        let place = Affine::translate(c.pos).then(&Affine::rot_y(yaw));
        let b = legs::Body {
            place,
            vel: c.vel,
            on_ground: c.on_ground,
            moving: c.gait != Gait::Still,
            now,
        };
        let stance = self.feet.update(&legs, &b, ground, &look.legs);
        Some(Legs {
            legs,
            stance,
            to_model: legs::rigid_inverse(&place),
        })
    }
}

/// Scales every part's transform by `s` about the model's origin (its
/// feet): the whole crab stretched or squashed where it stands.
pub fn squashed(transforms: &mut [crate::gfx::geom::Affine], s: V3) {
    if s == v3(ONE, ONE, ONE) {
        return;
    }
    let k = crate::gfx::geom::Affine::scale(s);
    for t in transforms.iter_mut() {
        *t = k.then(t);
    }
}

#[cfg(test)]
mod tests;
