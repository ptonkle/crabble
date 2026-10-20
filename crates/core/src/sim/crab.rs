//! A crab: where it stands, how it walks, jumps and lands, and the push
//! between two crabs that bump (PLAN.md, gameplay 4 and 8).
//!
//! It faces where its seat looks (CR-Q1): W and S shuffle it forward and
//! back, A and D scuttle it sideways. It walks up a step no higher than
//! `crab.step_mm` and jumps onto anything higher (a tile, the board's rim).

use serde::Serialize;

use crate::gfx::fx::ONE;
use crate::gfx::geom::{facing, v3, V3};
use crate::model::mm1;
use crate::online::CrabState;

use super::tiles::{Claw, TileId};

/// Who moves a crab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Driver {
    /// this browser's player
    Local,
    /// an AI crab, on this browser
    Ai,
    /// another browser, through its states
    Remote,
}

/// How a crab is walking, for its animation: not at all, forward or back
/// (W, S), or sideways to its left or right (A, D).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gait {
    Still,
    Forward,
    Back,
    Left,
    Right,
}

impl Gait {
    /// Its number in a crab's online state.
    pub fn code(self) -> u8 {
        match self {
            Gait::Still => 0,
            Gait::Forward => 1,
            Gait::Back => 2,
            Gait::Left => 3,
            Gait::Right => 4,
        }
    }

    /// The gait an online state's number names; an unknown one is still.
    pub fn from_code(code: u8) -> Gait {
        match code {
            1 => Gait::Forward,
            2 => Gait::Back,
            3 => Gait::Left,
            4 => Gait::Right,
            _ => Gait::Still,
        }
    }
}

/// A claw's last move, for its animation: a grab or a place, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    Grab,
    Place,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crab {
    pub driver: Driver,
    /// where its feet are (Q16 metres)
    pub pos: V3,
    /// Q16 metres a second
    pub vel: V3,
    /// which way it faces, and its seat's camera pitch (angle units)
    pub yaw: i32,
    pub pitch: i32,
    pub on_ground: bool,
    /// the tile in each claw, left and right
    pub claws: [Option<TileId>; 2],
    pub gait: Gait,
    /// how far its walk has gone and how long it has been in the air (ms),
    /// for its animations
    pub walk_ms: u32,
    pub air_ms: u32,
    /// each claw's last grab or place, and the game's clock (ms) then
    pub reach: [Option<(Reach, i64)>; 2],
    /// the game's clock (ms) when it last placed a tile (the start, until
    /// it has), for the idle penalty
    pub placed_ms: i64,
    /// idle penalty taken so far (points, at or below 0)
    pub penalty: i32,
}

impl Crab {
    pub fn new(driver: Driver, pos: V3, yaw: i32, pitch: i32) -> Crab {
        Crab {
            driver,
            pos,
            vel: V3::ZERO,
            yaw,
            pitch,
            on_ground: true,
            claws: [None; 2],
            gait: Gait::Still,
            walk_ms: 0,
            air_ms: 0,
            reach: [None; 2],
            placed_ms: 0,
            penalty: 0,
        }
    }

    /// Its state, for the online sync (mm, mm a second).
    pub fn state(&self) -> CrabState {
        // to the nearest mm, so a state read back in comes out the same
        let mm = |q: i32| (q as i64 * 1000 + ONE as i64 / 2).div_euclid(ONE as i64) as i32;
        CrabState {
            p: [mm(self.pos.x), mm(self.pos.z)],
            v: [mm(self.vel.x), mm(self.vel.z)],
            y: self.yaw,
            h: mm(self.pos.y),
            vh: mm(self.vel.y),
            a: !self.on_ground,
            g: self.gait.code(),
        }
    }

    /// Ahead of it, flat.
    pub fn forward(&self) -> V3 {
        facing(self.yaw, 0)
    }

    /// To its right, flat (the D key's way).
    pub fn right(&self) -> V3 {
        let f = self.forward();
        v3(-f.z, 0, f.x)
    }

    /// Where its claw tips are, on the sand under them.
    pub fn claw_tips(&self, claw_tip_mm: i32) -> V3 {
        self.pos.add(self.forward().scale(mm1(claw_tip_mm)))
    }

    pub fn holds(&self, claw: Claw) -> Option<TileId> {
        self.claws[claw]
    }
}

/// The walking velocity for keys `fwd` (+1 forward) and `side` (+1 right)
/// at `speed` (Q16 metres a second), facing `yaw`; a diagonal is no faster.
pub fn walk_velocity(yaw: i32, fwd: i32, side: i32, speed: i32) -> V3 {
    let f = facing(yaw, 0);
    let r = v3(-f.z, 0, f.x);
    let w = f.scale(fwd * ONE).add(r.scale(side * ONE));
    if w == V3::ZERO {
        return V3::ZERO;
    }
    w.with_len(speed)
}

/// The gait for keys `fwd` and `side`: a sideways key wins (a crab
/// scuttles).
pub fn gait(fwd: i32, side: i32) -> Gait {
    match (fwd.signum(), side.signum()) {
        (_, 1) => Gait::Right,
        (_, -1) => Gait::Left,
        (1, _) => Gait::Forward,
        (-1, _) => Gait::Back,
        _ => Gait::Still,
    }
}

/// How far two crabs at `a` and `b`, each `radius` round, are pushed
/// apart this tick: each moves `pct` percent of half their overlap, along
/// the line between them (flat). `None` when they do not touch.
pub fn push(a: V3, b: V3, radius: i32, pct: i32) -> Option<V3> {
    let d = b.sub(a).flat();
    let len = d.len();
    let overlap = 2 * radius - len;
    if overlap <= 0 {
        return None;
    }
    // two crabs exactly on top of each other part along x
    let dir = if len == 0 { v3(ONE, 0, 0) } else { d.with_len(ONE) };
    let shove = (overlap as i64 * pct as i64 / 200) as i32;
    Some(dir.with_len(shove.max(1)))
}
