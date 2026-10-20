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
//! eased lean, when it landed); `View3d` holds one per crab.

use serde::Deserialize;

use crate::gfx::fx::{self, deg, ONE, TURN};
use crate::gfx::geom::{facing, v3, V3};
use crate::model::{mm1, Model, Pose};
use crate::rng::mix;
use crate::sim::{Crab, Gait, Reach};

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
        let turn = frac(self.turn, fx::HALF);
        // rot_z: + tips the top toward -x (its right); rot_x: + toward +z
        let roll = -tilt(look.lean_deg, side) - tilt(look.turn_lean_deg, turn);
        let pitch = tilt(look.pitch_deg, fwd);
        [roll, pitch]
    }

    /// Adds this frame's extra motion to `poses` (the model's keyed moves
    /// for crab `c`) and returns the whole crab's scale (across, up,
    /// across; Q16) for its stretch or squash.
    pub fn dress(
        &self,
        c: &Crab,
        yaw: i32,
        poses: &mut [Pose],
        parts: &Parts,
        cx: &Context,
        look: &CrabLook,
    ) -> V3 {
        let now = cx.now;
        let walking = c.on_ground && c.gait != Gait::Still;
        let (side, fwd) = Rig::local_speed(c, yaw, cx.speed);
        let phase = (c.walk_ms as i64 * TURN as i64 / cx.cycle_ms.max(1) as i64) as i32;
        let turn = frac(self.turn, fx::HALF);

        // the body: a dip with each step, the lean
        let body = &mut poses[parts.body];
        if walking {
            body.shift.y -= fx::mul(mm1(look.bob_mm), fx::sin(phase).abs());
        }
        body.turn[2] += self.lean[0];
        body.turn[0] += self.lean[1];

        // the eyes trail the move and the turn, and nod on landing
        let nod = self
            .land
            .map(|(at, hard)| {
                let u = fx::ratio(
                    (now - at).clamp(0, 2 * look.squash_ms as i64) as i32,
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
            p.turn[2] += tilt(look.eye_lag_deg, side);
            p.turn[0] += -tilt(look.eye_lag_deg, fwd) + nod;
            p.turn[1] -= tilt(look.eye_lag_deg, turn);
        }

        // the claws sway with the walk; a grab squeezes, a place clacks
        let moving = frac(c.vel.flat().len(), cx.speed);
        for k in 0..2 {
            let held = c.claws[k].is_some();
            if let Some(a) = parts.arm[k] {
                let sway = fx::mul(tilt(look.claw_sway_deg, moving), fx::sin(phase));
                let sway = if held { sway / 2 } else { sway };
                // both arms swing the same way about their own y
                poses[a].turn[1] += sway;
                poses[a].turn[0] += fx::mul(tilt(look.claw_sway_deg, moving), fx::sin(2 * phase)) / 2;
            }
            if let Some((reach, at)) = c.reach[k] {
                let since = now - at;
                let snap = look.snap_ms as i64;
                match reach {
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
        self.fidget(c, poses, parts, cx, look);

        // stretch leaving the ground, squash landing
        let mut up = 0;
        if !c.on_ground && (c.air_ms as i32) < look.stretch_ms {
            let u = fx::ratio(c.air_ms as i32, look.stretch_ms);
            up = fx::mul(look.stretch_pct * ONE / 100, ONE - u);
        } else if let Some((at, hard)) = self.land.filter(|_| c.on_ground) {
            let since = now - at;
            if (0..look.squash_ms as i64).contains(&since) {
                let u = fx::ratio(since as i32, look.squash_ms);
                // down at once, back past even a little, and still
                let wave = fx::mul(ONE - u, fx::cos(u * 3 / 4));
                up = -fx::mul(fx::mul(look.squash_pct * ONE / 100, hard), wave);
            }
        }
        v3(ONE - up / 2, ONE + up, ONE - up / 2)
    }

    fn fidget(&self, c: &Crab, poses: &mut [Pose], parts: &Parts, cx: &Context, look: &CrabLook) {
        let still = cx.now - self.moved - look.fidget_after_ms as i64;
        if still < 0 || !c.on_ground || c.gait != Gait::Still {
            return;
        }
        let seat = cx.seat as u64;
        let every = look.fidget_every_ms as i64 + (mix(seat) % 700) as i64;
        let (round, within) = (still / every, still % every);
        if within >= look.fidget_ms as i64 {
            return;
        }
        let u = fx::ratio(within as i32, look.fidget_ms);
        // in and out over the fidget
        let env = fx::sin(u / 2);
        let d = deg(look.fidget_deg);
        match mix(seat * 977 + round as u64) % 3 {
            // looks about: both stalks one way, then the other
            0 => {
                for e in parts.eye.into_iter().flatten() {
                    poses[e].turn[1] += fx::mul(fx::mul(d, env), fx::sin(u));
                }
            }
            // clacks its empty claws, three times
            1 => {
                for k in 0..2 {
                    if let (Some(f), None) = (parts.finger[k], c.claws[k]) {
                        poses[f].turn[0] -= fx::mul(fx::mul(d, env), fx::sin(u * 3 / 2).abs());
                    }
                }
            }
            // shifts its weight from side to side
            _ => {
                let b = &mut poses[parts.body];
                b.turn[2] += fx::mul(d / 3, fx::mul(env, fx::sin(u)));
                b.shift.x += fx::mul(mm1(2), fx::mul(env, fx::sin(u)));
            }
        }
    }
}

#[cfg(test)]
mod tests;
