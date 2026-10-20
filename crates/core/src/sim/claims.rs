//! THE SEAM for the AI and online play. Every grab, placement and drop is
//! a [`Claim`] that one referee function, [`Match::judge`], checks; the
//! first claim wins (PLAN.md, gameplay 6; DECISIONS.md, 2026-10-19 online
//! sync).
//!
//! A click becomes a claim: a grab of the tile under the crosshair (empty
//! claw), the placing of a held tile on the legal square under it, or (a
//! full claw aimed anywhere else) a drop. `judge` grants a drop whenever
//! the claw still holds the tile. An AI crab makes the same clicks through
//! its `SeatInput`; it never writes claims itself.
//!
//! - **Local referee** (a local game, and an online host): the `Match`
//!   settles claims itself, each tick, in the order they came: first the
//!   claims other browsers sent ([`Match::submit_claim`]), then this tick's
//!   clicks in seat order. A claim on a tile no longer loose, a square
//!   already filled or no longer touching the others, or a claw no longer
//!   holding its tile, is refused. With `record_verdicts` on (a host),
//!   every verdict is kept for [`Match::take_verdicts`] to send.
//! - **Remote referee** (an online joiner): its own clicks go out through
//!   [`Match::take_claims`] and change nothing until the host's verdict
//!   comes back through [`Match::apply_verdict`]; a verdict on another
//!   seat's claim is applied the same way.

use crate::gfx::geom::{v3, Affine, V3};
use crate::model::mm1;

use super::aim::Target;
use super::crab::Reach;
use super::tiles::{Claw, State, TileId};
use super::{Event, Match, Rules, SeatInput, Square};

/// Who settles the claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Referee {
    /// this game itself
    Local,
    /// a host in another browser
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    Grab {
        seat: usize,
        claw: Claw,
        tile: TileId,
    },
    Place {
        seat: usize,
        claw: Claw,
        tile: TileId,
        square: Square,
    },
    Drop {
        seat: usize,
        claw: Claw,
        tile: TileId,
    },
}

impl Claim {
    /// The seat, claw and tile it is about.
    pub fn who(&self) -> (usize, Claw, TileId) {
        match *self {
            Claim::Grab { seat, claw, tile }
            | Claim::Place { seat, claw, tile, .. }
            | Claim::Drop { seat, claw, tile } => (seat, claw, tile),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Granted(Claim),
    Refused(Claim),
}

/// A tile's box frame on a square: flat on the board's top, square to it.
pub fn square_pose(r: &Rules, sq: Square) -> Affine {
    let mid = |k: i32| (k - r.board_size / 2) * r.square;
    Affine::translate(v3(mid(sq.1), r.board_top, mid(sq.0)))
}

impl Match {
    /// What a click of `claw` would do for seat `i` now: grab the tile in
    /// reach under the crosshair (empty claw), place its tile on the legal
    /// square in reach under it, or (a full claw aimed anywhere else) drop.
    pub fn click(&self, i: usize, claw: Claw) -> Option<Claim> {
        let aim = self.aim(i);
        match self.crabs[i].claws[claw] {
            None => match aim.target {
                Target::Tile(tile) if aim.in_reach => Some(Claim::Grab { seat: i, claw, tile }),
                _ => None,
            },
            Some(tile) => match aim.target {
                Target::Square(square) if aim.in_reach && self.legal_square(square) => Some(Claim::Place {
                    seat: i,
                    claw,
                    tile,
                    square,
                }),
                _ => Some(Claim::Drop { seat: i, claw, tile }),
            },
        }
    }

    /// Seat `i`'s clicks this tick: claims queued, drops done.
    pub(super) fn claws(&mut self, i: usize, input: &SeatInput) {
        for claw in [super::tiles::LEFT, super::tiles::RIGHT] {
            if !input.click[claw] {
                continue;
            }
            if let Some(c) = self.click(i, claw) {
                match self.referee {
                    super::Referee::Local => self.pending.push(c),
                    super::Referee::Remote => self.claims_out.push(c),
                }
            }
        }
    }

    /// The ghost tile seat `i` would place: the claw, its tile and the
    /// square, when a held tile is aimed at a legal square in reach. With
    /// a tile in each claw, the one held longer.
    pub fn ghost(&self, i: usize) -> Option<(Claw, TileId, Square)> {
        let c = self.crabs.get(i)?;
        let claw = match (c.claws[0], c.claws[1]) {
            (Some(a), Some(b)) => {
                let since = |t: TileId| self.tiles.get(t).map_or(0, |t| t.since_ms);
                if since(b) < since(a) {
                    1
                } else {
                    0
                }
            }
            (Some(_), None) => 0,
            (None, Some(_)) => 1,
            (None, None) => return None,
        };
        let aim = self.aim(i);
        match aim.target {
            Target::Square(sq) if aim.in_reach && self.legal_square(sq) => Some((claw, c.claws[claw]?, sq)),
            _ => None,
        }
    }

    /// A claim from another browser, for a host to settle next tick.
    pub fn submit_claim(&mut self, c: Claim) {
        self.claims_in.push(c);
    }

    /// A joiner's claims for the host.
    pub fn take_claims(&mut self) -> Vec<Claim> {
        std::mem::take(&mut self.claims_out)
    }

    /// A host's verdicts for everyone.
    pub fn take_verdicts(&mut self) -> Vec<Verdict> {
        std::mem::take(&mut self.verdicts)
    }

    /// Settles this tick's claims, first come first served (a local
    /// referee); a remote referee settles nothing here.
    pub(super) fn settle_claims(&mut self) {
        let mut queue = std::mem::take(&mut self.claims_in);
        queue.append(&mut self.pending);
        for c in queue {
            let v = if self.judge(&c) {
                let now = self.play_ms();
                let drop = self.grant_at(c, now, None);
                if self.record_verdicts {
                    self.record_judged(Verdict::Granted(c), now, drop);
                }
                Verdict::Granted(c)
            } else {
                let (seat, claw, tile) = c.who();
                self.events.push(Event::Refused { seat, claw, tile });
                if self.record_verdicts {
                    self.record_judged(Verdict::Refused(c), self.play_ms(), None);
                }
                Verdict::Refused(c)
            };
            if self.record_verdicts {
                self.verdicts.push(v);
            }
        }
    }

    /// The host's verdict, on a joiner: a granted claim happens here as it
    /// did there; a refused one of this browser's is heard.
    pub fn apply_verdict(&mut self, v: Verdict) {
        match v {
            Verdict::Granted(c) => {
                self.grant_at(c, self.play_ms(), None);
            }
            Verdict::Refused(c) => {
                let (seat, claw, tile) = c.who();
                self.events.push(Event::Refused { seat, claw, tile });
            }
        }
    }

    /// The referee: whether a claim can still happen, judged against the
    /// game as it is now (so of two claims on one tile or square, the
    /// first judged wins). A grab needs its tile loose and the claw empty;
    /// a place needs the claw holding its tile and the square legal; a
    /// drop needs the claw holding its tile.
    pub fn judge(&self, c: &Claim) -> bool {
        let Some(crab) = self.crabs.get(c.who().0) else {
            return false;
        };
        let Some(t) = self.tiles.get(c.who().2) else {
            return false;
        };
        match *c {
            Claim::Grab { claw, .. } => t.is_loose() && crab.claws[claw].is_none(),
            Claim::Place {
                claw, tile, square, ..
            } => crab.claws[claw] == Some(tile) && self.legal_square(square),
            Claim::Drop { claw, tile, .. } => crab.claws[claw] == Some(tile),
        }
    }

    /// A claim happens at game time `now` (ms; online, the host's time).
    /// A drop lands at `drop` when given (the host's pose), else where
    /// the dropping browser said (`drop_at`), else ahead of the claw; the
    /// pose it lands at comes back.
    pub(super) fn grant_at(&mut self, c: Claim, now: i64, drop: Option<Affine>) -> Option<Affine> {
        match c {
            Claim::Grab { seat, claw, tile } => {
                let t = &mut self.tiles.all[tile as usize];
                t.from = t.draw_pose();
                t.state = State::Held { seat, claw };
                t.since_ms = now;
                t.vy = 0;
                t.slide = V3::ZERO;
                self.crabs[seat].claws[claw] = Some(tile);
                self.crabs[seat].reach[claw] = Some((Reach::Grab, now));
                self.tiles.wake_above(tile);
                self.events.push(Event::Grabbed { seat, claw, tile });
            }
            Claim::Place {
                seat,
                claw,
                tile,
                square,
            } => {
                let pose = square_pose(&self.rules, square);
                let order = self.placements;
                self.placements += 1;
                let t = &mut self.tiles.all[tile as usize];
                t.state = State::Placed {
                    square,
                    seat: Some(seat),
                    order,
                };
                t.pose = pose;
                t.face_down = false;
                t.since_ms = now;
                let c = &mut self.crabs[seat];
                c.claws[claw] = None;
                c.reach[claw] = Some((Reach::Place, now));
                c.placed_ms = now;
                self.events.push(Event::Placed {
                    seat,
                    claw,
                    tile,
                    square,
                });
            }
            Claim::Drop { seat, claw, .. } => {
                let pose = drop.or_else(|| self.take_drop_at(seat, claw));
                return self.drop_tile(seat, claw, now, pose);
            }
        }
        None
    }

    /// Where seat `i`'s tile would fall if it let go now: just ahead of
    /// its claw tips.
    pub fn drop_pose(&self, i: usize) -> Affine {
        let r = &self.rules;
        let c = &self.crabs[i];
        let at = c
            .claw_tips(r.claw_tip + r.drop_ahead + mm1(super::tiles::SIZE_MM / 2))
            .add(v3(0, r.drop_up, 0));
        Affine::translate(at).then(&Affine::rot_y(c.yaw))
    }

    /// Seat `i`'s claw lets its tile fall at `pose` (or just ahead of its
    /// claw tips) at game time `now` (a granted drop); the pose comes back.
    fn drop_tile(&mut self, i: usize, claw: Claw, now: i64, pose: Option<Affine>) -> Option<Affine> {
        let pose = pose.unwrap_or_else(|| self.drop_pose(i));
        let tile = self.crabs[i].claws[claw].take()?;
        let t = &mut self.tiles.all[tile as usize];
        t.state = State::Loose { awake: true };
        t.pose = pose;
        t.face_down = false;
        t.vy = 0;
        t.awake_ticks = 0;
        t.since_ms = now;
        t.rests_on.clear();
        self.events.push(Event::Dropped { seat: i, claw, tile });
        Some(pose)
    }
}
