//! The game's side of the online tile sync (`online/tiles.rs`;
//! SECOND-ORDER R5.5-R5.12): an online host's game keeps its tile changes
//! in the order they happen ([`Change`]), and a joiner's game applies the
//! same changes in the same order, with the host's times and poses, so the
//! held tiles, the placements (owner, order, time), the idle timers and the
//! board come out the same on every browser.
//!
//! A joiner's game still runs the piles' physics itself, so a tumble looks
//! smooth there; each tile that comes to rest on the host is sent with its
//! pose and the tiles under it, and the joiner's copy is put where the
//! host's lies ([`Match::apply_settled`]).

use crate::gfx::geom::Affine;

use super::claims::Verdict;
use super::tiles::{Claw, State, TileId};
use super::Match;

/// One change to the tiles on an online host, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// The referee judged a claim at game time `at_ms`; a granted drop
    /// fell at `drop`.
    Judged {
        verdict: Verdict,
        at_ms: i64,
        drop: Option<Affine>,
    },
    /// A loose tile came to rest: its pose, its face, the tiles under it.
    Settled {
        tile: TileId,
        pose: Affine,
        face_down: bool,
        on: Vec<TileId>,
    },
}

impl Match {
    pub(super) fn record_judged(&mut self, verdict: Verdict, at_ms: i64, drop: Option<Affine>) {
        self.changes.push(Change::Judged { verdict, at_ms, drop });
    }

    pub(super) fn record_settled(&mut self, settled: Vec<TileId>) {
        for id in settled {
            if let Some(t) = self.tiles.get(id) {
                let c = Change::Settled {
                    tile: id,
                    pose: t.pose,
                    face_down: t.face_down,
                    on: t.rests_on.clone(),
                };
                self.changes.push(c);
            }
        }
    }

    /// An online host's tile changes since the last call.
    pub fn take_changes(&mut self) -> Vec<Change> {
        std::mem::take(&mut self.changes)
    }

    /// Host: where a joiner's browser dropped seat `seat`'s tile from claw
    /// `claw`, for its Drop claim (that browser moves the crab).
    pub fn submit_drop_at(&mut self, seat: usize, claw: Claw, pose: Affine) {
        self.drop_at.retain(|d| (d.0, d.1) != (seat, claw));
        self.drop_at.push((seat, claw, pose));
    }

    pub(super) fn take_drop_at(&mut self, seat: usize, claw: Claw) -> Option<Affine> {
        let k = self.drop_at.iter().position(|d| (d.0, d.1) == (seat, claw))?;
        Some(self.drop_at.remove(k).2)
    }

    /// Joiner: the host's verdict, at the host's game time, a drop at the
    /// host's pose. A granted claim happens here as it did there; a refused
    /// one is heard (and changes nothing: nothing was shown before it).
    pub fn apply_judged(&mut self, verdict: Verdict, at_ms: i64, drop: Option<Affine>) {
        match verdict {
            Verdict::Granted(c) => {
                self.grant_at(c, at_ms, drop);
            }
            Verdict::Refused(_) => self.apply_verdict(verdict),
        }
    }

    /// Joiner: a tile came to rest on the host. A loose tile here is put
    /// where the host's lies, asleep; a tile held or placed here since is
    /// left alone (its later change came in the host's order too).
    pub fn apply_settled(&mut self, tile: TileId, pose: Affine, face_down: bool, on: Vec<TileId>) {
        let Some(t) = self.tiles.all.get_mut(tile as usize) else {
            return;
        };
        if !t.is_loose() {
            return;
        }
        t.state = State::Loose { awake: false };
        t.pose = pose;
        t.face_down = face_down;
        t.vy = 0;
        t.slide = crate::gfx::geom::V3::ZERO;
        t.rests_on = on;
    }

    /// The end, on a joiner: each seat's idle penalty as the host counted
    /// it.
    pub fn set_penalties(&mut self, penalties: &[i32]) {
        for (c, p) in self.crabs.iter_mut().zip(penalties) {
            c.penalty = *p;
        }
    }
}
