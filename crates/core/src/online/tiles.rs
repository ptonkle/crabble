//! Tile sync: the host's tiles (DECISIONS.md, 2026-10-19 online sync;
//! `CLAUDE.md`, "Online tiles are the host's"; SECOND-ORDER R5.5-R5.12).
//!
//! # A browser asks, the host decides
//!
//! A joiner's grab, placement or drop is a [`WireClaim`] sent to the host
//! with the joiner's own number `n` (1, 2, 3 ...), repeated every
//! [`super::RETRY_MS`] until its verdict is heard. The host takes each
//! joiner's claims in their numbers' order (a later one waits for an
//! earlier one lost on the way; a repeat is dropped), and its `Match`
//! settles them with its own clicks and its AI crabs': the first claim the
//! host's referee judges wins.
//!
//! # One numbered log
//!
//! Every verdict (granted or refused, with the host's game clock then) and
//! every tile that came to rest on the host (its pose, face, and the tiles
//! it lies on) is an [`Entry`] in one log, numbered from 0 in the order the
//! host's game made them. The host sends each new entry to everyone at
//! once ([`Msg::Log`](super::msg::Msg)), and sends the rest again to a
//! joiner whose Have (the next number it needs, every
//! [`HAVE_EVERY_MS`]) is behind. A joiner applies the entries strictly in
//! number order, holding any that come early, so every browser's game goes
//! through the same changes in the same order: the same tiles held in each
//! claw, the same placements with the same owners, place order and times,
//! the same idle timers, and the same board at the end.
//!
//! # The end
//!
//! The host's End carries the log's length and each seat's idle penalty: a
//! joiner's game ends only once it has applied every entry before it
//! (unless the host has gone), and takes the host's penalties.

use serde::{Deserialize, Serialize};

/// What a claim asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Grab,
    Place,
    Drop,
}

/// A claim on the wire: kind, seat, claw, tile; a placement's square (row,
/// column); a drop's pose, as the dropping crab's own browser saw it (that
/// browser moves the crab, so its claws are where it says).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireClaim {
    pub k: Kind,
    pub s: usize,
    pub c: usize,
    pub t: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q: Option<[i32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<[i32; 12]>,
}

/// One change to the tiles, in the host's order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "e", rename_all = "snake_case")]
pub enum Entry {
    /// The referee's answer to a claim: `ok` granted, `at` the host's game
    /// clock (ms) then, `n` the claimant's number for it (0 for a claim
    /// made on the host: its player's or an AI crab's).
    Verdict { ok: bool, c: WireClaim, n: u32, at: i64 },
    /// A loose tile came to rest on the host: its pose (Q16, column by
    /// column), face down or not, and the tiles under it.
    Settle {
        t: u16,
        pose: [i32; 12],
        fd: bool,
        on: Vec<u16>,
    },
}

/// How often a joiner says which log entry it needs next.
pub const HAVE_EVERY_MS: i64 = 250;
/// The most entries the host sends again in one message.
pub const RESEND_MAX: usize = 64;

use std::collections::{BTreeMap, VecDeque};

use super::PeerId;

/// One game's tile sync, on the host or a joiner (`Session` drives it).
#[derive(Clone, Debug, Default)]
pub(crate) struct TileSync {
    // ---- the host ----
    /// every entry so far, and when each was last sent to everyone
    pub(crate) log: Vec<Entry>,
    sent_ms: Vec<i64>,
    /// each joiner's next needed entry, and when it was last sent the rest
    have: BTreeMap<PeerId, (u64, i64)>,
    /// each joiner seat's next claim number, and the claims come early
    incoming: BTreeMap<usize, (u32, BTreeMap<u32, WireClaim>)>,
    /// joiners' claims in order, for the referee
    claims: Vec<WireClaim>,
    /// the numbers of the claims handed to the referee, per seat, in order
    numbers: BTreeMap<usize, VecDeque<u32>>,
    // ---- a joiner ----
    /// the next entry to apply, the ones come early, and the ones in order
    /// not yet taken
    pub(crate) next: u64,
    early: BTreeMap<u64, Entry>,
    ready: Vec<Entry>,
    /// this browser's last claim number, and its claims with no verdict yet
    /// (with when each was last sent)
    my_n: u32,
    pending: BTreeMap<u32, (WireClaim, i64)>,
    pub(crate) last_have_ms: i64,
    /// from the host's End: the log's length and the idle penalties
    pub(crate) end_n: u64,
    pub(crate) end_pen: Vec<i32>,
}

impl TileSync {
    pub(crate) fn new(now_ms: i64) -> TileSync {
        TileSync {
            last_have_ms: now_ms,
            ..TileSync::default()
        }
    }

    // ---- the host ----

    /// A joiner's claim number `n` for seat `seat`; repeats are dropped,
    /// and a claim come early waits for those before it.
    pub(crate) fn on_claim(&mut self, seat: usize, n: u32, mut c: WireClaim) {
        c.s = seat;
        let (next, early) = self.incoming.entry(seat).or_insert((1, BTreeMap::new()));
        if n < *next {
            return;
        }
        early.insert(n, c);
        while let Some(c) = early.remove(&*next) {
            self.claims.push(c);
            self.numbers.entry(seat).or_default().push_back(*next);
            *next += 1;
        }
    }

    pub(crate) fn take_claims(&mut self) -> Vec<WireClaim> {
        std::mem::take(&mut self.claims)
    }

    /// Appends the host game's new entries; gives the first one's number.
    /// A verdict on a joiner's claim takes that claim's number.
    pub(crate) fn record(&mut self, entries: Vec<Entry>, now_ms: i64) -> u64 {
        let from = self.log.len() as u64;
        for mut e in entries {
            if let Entry::Verdict { c, n, .. } = &mut e {
                if let Some(q) = self.numbers.get_mut(&c.s) {
                    *n = q.pop_front().unwrap_or(0);
                }
            }
            self.log.push(e);
            self.sent_ms.push(now_ms);
        }
        from
    }

    pub(crate) fn on_have(&mut self, peer: &PeerId, n: u64) {
        let e = self.have.entry(peer.clone()).or_insert((0, i64::MIN / 2));
        e.0 = e.0.max(n);
    }

    pub(crate) fn forget(&mut self, peer: &PeerId) {
        self.have.remove(peer);
    }

    /// The entries to send again: to each joiner behind, those it lacks
    /// that went out at least `retry_ms` ago, at most [`RESEND_MAX`].
    pub(crate) fn resends(&mut self, now_ms: i64, retry_ms: i64) -> Vec<(PeerId, u64, Vec<Entry>)> {
        let mut out = Vec::new();
        let len = self.log.len() as u64;
        for (peer, (have, last)) in self.have.iter_mut() {
            if *have >= len || now_ms - *last < retry_ms {
                continue;
            }
            let from = *have as usize;
            let upto = (from..self.log.len())
                .take(RESEND_MAX)
                .take_while(|k| now_ms - self.sent_ms[*k] >= retry_ms)
                .last();
            if let Some(upto) = upto {
                *last = now_ms;
                out.push((peer.clone(), *have, self.log[from..=upto].to_vec()));
            }
        }
        out
    }

    // ---- a joiner ----

    /// Whether this browser has a claim for claw `claw` with no verdict yet.
    pub(crate) fn in_flight(&self, claw: usize) -> bool {
        self.pending.values().any(|(c, _)| c.c == claw)
    }

    /// Numbers a new claim and keeps it until its verdict.
    pub(crate) fn claim(&mut self, c: WireClaim, now_ms: i64) -> u32 {
        self.my_n += 1;
        self.pending.insert(self.my_n, (c, now_ms));
        self.my_n
    }

    /// The claims to send again.
    pub(crate) fn claim_resends(&mut self, now_ms: i64, retry_ms: i64) -> Vec<(u32, WireClaim)> {
        self.pending
            .iter_mut()
            .filter(|(_, (_, at))| now_ms - *at >= retry_ms)
            .map(|(n, (c, at))| {
                *at = now_ms;
                (*n, *c)
            })
            .collect()
    }

    /// Entries from the host, from number `from` on. True when one is
    /// still missing before those held (send a Have now).
    pub(crate) fn on_log(&mut self, from: u64, entries: Vec<Entry>, my_seat: usize) -> bool {
        for (k, e) in entries.into_iter().enumerate() {
            let seq = from + k as u64;
            if seq >= self.next {
                self.early.insert(seq, e);
            }
        }
        while let Some(e) = self.early.remove(&self.next) {
            if let Entry::Verdict { c, n, .. } = &e {
                if c.s == my_seat && *n > 0 {
                    self.pending.remove(n);
                }
            }
            self.ready.push(e);
            self.next += 1;
        }
        !self.early.is_empty()
    }

    /// The entries in order, to apply to this browser's game.
    pub(crate) fn take_entries(&mut self) -> Vec<Entry> {
        std::mem::take(&mut self.ready)
    }
}

// ---- between the wire and the game (`sim::sync`) ----

use crate::gfx::geom::{v3, Affine};
use crate::sim::{Change, Claim, Match, Verdict};

/// A pose as twelve numbers, column by column.
pub fn pose_out(a: &Affine) -> [i32; 12] {
    let [c0, c1, c2, t] = [a.c0, a.c1, a.c2, a.t];
    [
        c0.x, c0.y, c0.z, c1.x, c1.y, c1.z, c2.x, c2.y, c2.z, t.x, t.y, t.z,
    ]
}

pub fn pose_in(p: &[i32; 12]) -> Affine {
    Affine {
        c0: v3(p[0], p[1], p[2]),
        c1: v3(p[3], p[4], p[5]),
        c2: v3(p[6], p[7], p[8]),
        t: v3(p[9], p[10], p[11]),
    }
}

impl WireClaim {
    /// A game's claim for the wire; a drop carries `drop`, where it fell.
    pub fn from_claim(c: &Claim, drop: Option<Affine>) -> WireClaim {
        let (s, claw, t) = c.who();
        let (k, q) = match *c {
            Claim::Grab { .. } => (Kind::Grab, None),
            Claim::Place { square, .. } => (Kind::Place, Some([square.0, square.1])),
            Claim::Drop { .. } => (Kind::Drop, None),
        };
        WireClaim {
            k,
            s,
            c: claw,
            t,
            q,
            at: drop.as_ref().map(pose_out),
        }
    }

    /// The game's claim; None for a claim that does not make sense (a
    /// claw other than 0 or 1, a placement with no square).
    pub fn to_claim(&self) -> Option<Claim> {
        let (seat, claw, tile) = (self.s, self.c, self.t);
        if claw > 1 {
            return None;
        }
        Some(match self.k {
            Kind::Grab => Claim::Grab { seat, claw, tile },
            Kind::Place => {
                let q = self.q?;
                Claim::Place {
                    seat,
                    claw,
                    tile,
                    square: (q[0], q[1]),
                }
            }
            Kind::Drop => Claim::Drop { seat, claw, tile },
        })
    }
}

impl Entry {
    /// A host game's change as a log entry (its claim number filled in by
    /// the session).
    pub fn from_change(ch: &Change) -> Entry {
        match ch {
            Change::Judged { verdict, at_ms, drop } => {
                let (ok, c) = match verdict {
                    Verdict::Granted(c) => (true, c),
                    Verdict::Refused(c) => (false, c),
                };
                Entry::Verdict {
                    ok,
                    c: WireClaim::from_claim(c, *drop),
                    n: 0,
                    at: *at_ms,
                }
            }
            Change::Settled {
                tile,
                pose,
                face_down,
                on,
            } => Entry::Settle {
                t: *tile,
                pose: pose_out(pose),
                fd: *face_down,
                on: on.clone(),
            },
        }
    }

    /// A joiner applies the host's entry to its game.
    pub fn apply(&self, g: &mut Match) {
        match self {
            Entry::Verdict { ok, c, at, .. } => {
                let Some(claim) = c.to_claim() else { return };
                let v = if *ok {
                    Verdict::Granted(claim)
                } else {
                    Verdict::Refused(claim)
                };
                g.apply_judged(v, *at, c.at.as_ref().map(pose_in));
            }
            Entry::Settle { t, pose, fd, on } => g.apply_settled(*t, pose_in(pose), *fd, on.clone()),
        }
    }
}
