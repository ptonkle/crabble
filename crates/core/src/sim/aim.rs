//! The aim (PLAN.md, gameplay 10): the camera's centre ray is the
//! crosshair. It picks the first loose tile it meets, or the board square
//! it meets; either counts only within reach of the crab's claws:
//! `crab.reach_mm` past its claw tips, and no more than `crab.reach_up_mm`
//! over its feet.

use crate::gfx::fx;
use crate::gfx::geom::{v3, V3};
use crate::model::mm1;

use super::tiles::{State, TileId, SIZE_MM, THICK_MM};
use super::{Match, Square};

/// What the crosshair is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Tile(TileId),
    Square(Square),
    Nothing,
}

/// The crosshair's target, where the ray meets it, and whether the crab
/// can reach it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aim {
    pub target: Target,
    pub point: V3,
    pub in_reach: bool,
}

/// Where a ray from `o` along unit `d` first meets the box of `pose`
/// (Q16 metres along it), if it does.
fn ray_box(o: V3, d: V3, pose: &crate::gfx::geom::Affine) -> Option<i32> {
    // into the box's frame: its axes are unit and square to each other
    let rel = o.sub(pose.t);
    let lo_ = [o_dot(rel, pose.c0), o_dot(rel, pose.c1), o_dot(rel, pose.c2)];
    let dir = [o_dot(d, pose.c0), o_dot(d, pose.c1), o_dot(d, pose.c2)];
    let h = mm1(SIZE_MM / 2);
    let lo = [-h, 0, -h];
    let hi = [h, mm1(THICK_MM), h];
    let (mut near, mut far) = (i32::MIN, i32::MAX);
    for k in 0..3 {
        if dir[k] == 0 {
            if lo_[k] < lo[k] || lo_[k] > hi[k] {
                return None;
            }
            continue;
        }
        let a = fx::div(lo[k] - lo_[k], dir[k]);
        let b = fx::div(hi[k] - lo_[k], dir[k]);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near <= far && far >= 0).then_some(near.max(0))
}

fn o_dot(a: V3, b: V3) -> i32 {
    a.dot(b)
}

impl Match {
    /// The square under a point on the board, if it is on the board.
    pub fn square_at(&self, p: V3) -> Option<Square> {
        let r = &self.rules;
        let cell = |v: i32| (v + r.board_half).div_euclid(r.square.max(1));
        let (row, col) = (cell(p.z), cell(p.x));
        let on = |k: i32| (0..r.board_size).contains(&k);
        (on(row) && on(col)).then_some((row, col))
    }

    /// The tile placed on a square, if any.
    pub fn placed_at(&self, sq: Square) -> Option<TileId> {
        self.tiles
            .all
            .iter()
            .find(|t| matches!(t.state, State::Placed { square, .. } if square == sq))
            .map(|t| t.id)
    }

    /// Whether a tile may go on a square: it is empty and shares a side
    /// with a tile already there (BRIEF.md, Board).
    pub fn legal_square(&self, sq: Square) -> bool {
        let n = self.rules.board_size;
        let on = |(r, c): Square| (0..n).contains(&r) && (0..n).contains(&c);
        on(sq)
            && self.placed_at(sq).is_none()
            && [(-1, 0), (1, 0), (0, -1), (0, 1)]
                .iter()
                .any(|(dr, dc)| self.placed_at((sq.0 + dr, sq.1 + dc)).is_some())
    }

    /// Whether seat `i`'s crab can reach a point: within reach of its claw
    /// tips (taken a little over the ground under them), and not too far
    /// up.
    pub fn in_reach(&self, i: usize, p: V3) -> bool {
        let c = &self.crabs[i];
        let r = &self.rules;
        let tips = c.claw_tips(r.claw_tip);
        let tips = v3(tips.x, c.pos.y + mm1(20), tips.z);
        p.sub(tips).len() <= r.reach && p.y - c.pos.y <= r.reach_up
    }

    /// What a ray from `eye` along `fwd` (the camera's centre) is on, for
    /// seat `i`'s crab.
    pub fn aim_from(&self, i: usize, eye: V3, fwd: V3) -> Aim {
        let d = fwd.norm();
        let longest = self.rules.cam_distance + self.rules.reach * 4;
        let mut best: Option<(i32, TileId)> = None;
        for t in &self.tiles.all {
            if !t.is_loose() {
                continue;
            }
            // only tiles near the crab can be in reach
            let c = &self.crabs[i];
            if t.pose
                .t
                .sub(c.pos)
                .flat()
                .longer_than(self.rules.reach * 3 + mm1(SIZE_MM))
            {
                continue;
            }
            if let Some(s) = ray_box(eye, d, &t.pose) {
                if s <= longest && best.is_none_or(|(b, _)| s < b) {
                    best = Some((s, t.id));
                }
            }
        }
        // the board's top
        let board = if d.y < 0 {
            let s = fx::div(self.rules.board_top - eye.y, d.y);
            let p = eye.add(d.scale(s));
            self.square_at(p)
                .filter(|_| s >= 0 && s <= longest)
                .map(|sq| (s, sq, p))
        } else {
            None
        };
        let (target, point) = match (best, board) {
            (Some((s, id)), Some((b, _, _))) if s <= b => (Target::Tile(id), eye.add(d.scale(s))),
            (Some((s, id)), None) => (Target::Tile(id), eye.add(d.scale(s))),
            (_, Some((_, sq, p))) => (Target::Square(sq), p),
            (None, None) => (Target::Nothing, eye.add(d.scale(longest))),
        };
        Aim {
            target,
            point,
            in_reach: target != Target::Nothing && self.in_reach(i, point),
        }
    }

    /// Seat `i`'s aim, from its camera as of its last tick.
    pub fn aim(&self, i: usize) -> Aim {
        let (eye, fwd) = self.camera(i);
        self.aim_from(i, eye, fwd)
    }
}
