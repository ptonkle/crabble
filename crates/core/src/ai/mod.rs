//! The AI crabs (BRIEF.md, AI crabs): one medium level, for automated
//! play-throughs and balancing. An AI crab plays through exactly what a
//! person has: each tick [`think`] turns its seat's view of the game into
//! one `SeatInput` (keys held, the camera's look, a claw's click). It never
//! moves a crab or a tile itself.
//!
//! What it knows is what a crab could see: the board, and the letters of
//! face-up loose tiles within `ai.see_mm` of it. A face-down tile shows no
//! letter; it may still grab one blind. The board it plans on is the board
//! as it last saw it, from within `ai.see_mm` of the board's edge.
//!
//! It thinks in the game's own time (`ai` in `data/tuning.json`): after
//! each step it waits `react_ms`; at a pile it takes `letter_ms` (and
//! `per_letter_ms` for each letter in view) to choose a letter, and
//! `plan_ms` to work out the word it is for; at the board `place_ms` to
//! settle the square. Each is stretched or shortened by up to `jitter_pct`
//! percent. Its camera turns at most `turn_deg_per_s`, and its aim starts
//! up to `aim_error_mm` off and halves the miss every `aim_settle_ms`; it
//! clicks only when the crosshair is on what it means.
//!
//! The cycle: walk to the nearest pile, choose a letter in view (the one
//! whose best real word is worth most, less the walk, a climb up a pile
//! counted three times; tiles out of reach up a pile and tiles with another
//! lying on them are left), fetch it, look once more for a second letter
//! while a claw is free, walk to the square its plan named, re-plan with
//! the board as it is now, place. Fetching, it takes another tile its
//! crosshair lands on when it is as good. A goal it comes no closer to in
//! 3 s it gives up, and wanders a random way for a moment to come loose.
//! A tile with no real word to make waits for a second; two such tiles,
//! and it drops the less common. Past `hurry_ms` without placing (the idle
//! penalty starts at 30 s, BRIEF.md) it places the best it holds even if
//! the word is fake, and with empty claws grabs the nearest tile.
//! Growing another crab's real word takes it (a steal), and the planner
//! counts what the other crab loses (`steal_pct`).

pub mod plan;

use serde::Deserialize;

use crate::gfx::fx::{self, deg};
use crate::gfx::geom::{pitch_of, v3, yaw_of, V3};
use crate::model::mm1;
use crate::rng::Dice;
use crate::sim::aim::Target;
use crate::sim::tiles::THICK_MM;
use crate::sim::{Claim, Claw, Match, Phase, SeatInput, Square, State, TileId};

use plan::{Board, Seen};

/// The AI's numbers (`ai` in `data/tuning.json`), one medium level.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Numbers {
    /// it reads face-up letters and the board this far away
    pub see_mm: i32,
    /// the pause after each step before the next
    pub react_ms: i32,
    /// choosing a letter in view, and more for each different letter
    pub letter_ms: i32,
    pub per_letter_ms: i32,
    /// the word plan: where the chosen letter will go
    pub plan_ms: i32,
    /// at the board, settling the square for what it holds
    pub place_ms: i32,
    /// each think is this many percent longer or shorter at most
    pub jitter_pct: i32,
    /// the fastest its camera turns
    pub turn_deg_per_s: i32,
    /// how far off its aim starts, and how often the miss halves
    pub aim_error_mm: i32,
    pub aim_settle_ms: i32,
    /// this long without placing, it places whatever it holds
    pub hurry_ms: i32,
    /// a stolen word is worth this many percent of its score more
    pub steal_pct: i32,
    /// a letter's worth falls this many points a metre of walk to it
    pub walk_cost_per_m: i32,
    /// it gives up on a tile or a square it has not managed in this long
    pub give_up_ms: i32,
}

impl Numbers {
    /// Each value's range, by name, for the tuning file's check.
    pub fn check(&self) -> Result<(), String> {
        let checks: [(&str, i32, i32, i32); 14] = [
            ("ai.see_mm", self.see_mm, 100, 10_000),
            ("ai.react_ms", self.react_ms, 0, 10_000),
            ("ai.letter_ms", self.letter_ms, 0, 20_000),
            ("ai.per_letter_ms", self.per_letter_ms, 0, 5000),
            ("ai.plan_ms", self.plan_ms, 0, 20_000),
            ("ai.place_ms", self.place_ms, 0, 20_000),
            ("ai.jitter_pct", self.jitter_pct, 0, 90),
            ("ai.turn_deg_per_s", self.turn_deg_per_s, 10, 3600),
            ("ai.aim_error_mm", self.aim_error_mm, 0, 1000),
            ("ai.aim_settle_ms", self.aim_settle_ms, 20, 5000),
            ("ai.hurry_ms", self.hurry_ms, 0, 120_000),
            ("ai.steal_pct", self.steal_pct, 0, 300),
            ("ai.walk_cost_per_m", self.walk_cost_per_m, 0, 100),
            ("ai.give_up_ms", self.give_up_ms, 500, 60_000),
        ];
        for (name, value, lo, hi) in checks {
            if value < lo || value > hi {
                return Err(format!(
                    "The tuning file's {name} is {value}; it must be {lo} to {hi}."
                ));
            }
        }
        Ok(())
    }
}

/// A click waits this long for its answer before the claw clicks again.
const CLICK_WAIT_MS: i64 = 300;
/// Walking, it checks its progress this often; under a quarter of the way
/// it should have gone, it is stuck: it jumps, then steps aside.
const STUCK_MS: i64 = 400;
const SIDESTEP_MS: i64 = 600;
/// Walking, it gives a goal up when it has come no closer by this in this
/// long (a pile it cannot climb into, a crab in the way).
const PROGRESS_MM: i32 = 100;
const STALL_MS: i64 = 3000;
/// It reads only tiles at least this far inside its reach up.
const HIGH_MARGIN_MM: i32 = 60;
/// After a goal it could not reach it wanders a random way this long.
const WANDER_MS: i64 = 1500;
/// It looks for a second letter only with this long to spare before
/// `hurry_ms`.
const SECOND_LOOK_SPARE_MS: i64 = 8000;
/// Fetching a tile this long, it takes any good tile its crosshair is on.
const SWITCH_MS: i64 = 2000;
/// It walks forward only when facing within this of its way.
const WALK_ARC: i32 = deg(60);

/// What it is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Doing {
    /// between steps: decides the next once `busy_until` has passed
    Next,
    /// walking to a pile, toward this tile
    ToPile { tile: TileId },
    /// reading the letters in view and planning, until the clock says
    Choose { until: i64, hurry: bool },
    /// walking to a tile, aiming at it and clicking a claw
    Fetch { tile: TileId, claw: Claw, since: i64 },
    /// walking to the board, toward a square
    ToBoard { square: Square },
    /// settling the square, until the clock says
    Settle { until: i64 },
    /// walking to a square, aiming at it and clicking the claw
    Place { claw: Claw, square: Square, since: i64 },
    /// dropping a claw's tile (aimed off the board)
    Drop { claw: Claw, since: i64 },
}

/// Something it decided, and when (the game's clock, ms): for the tests
/// and the playtest's numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Thought {
    /// chose a tile to fetch; `planned` when a real word was in mind
    Chose {
        ms: i64,
        tile: TileId,
        letter: Option<char>,
        planned: bool,
    },
    /// clicked to place a letter on a square: its words real or not, a
    /// steal or not
    Placing {
        ms: i64,
        square: Square,
        letter: char,
        real: bool,
        steal: bool,
    },
    /// let a tile go that it had no word for
    Dropping { ms: i64, letter: char },
    /// gave up on a tile or a square
    /// `what` names the step: to_pile, fetch, to_board, place, drop
    GaveUp { ms: i64, what: &'static str },
}

/// One AI crab's mind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brain {
    n: Numbers,
    dice: Dice,
    pub doing: Doing,
    busy_until: i64,
    /// the board as it last saw it
    pub board: Board,
    /// the square its word plan named, if any
    planned: Option<Square>,
    /// where its camera looks (what it sends)
    yaw: i32,
    pitch: i32,
    started: bool,
    /// its aim's miss, and when it last halved
    miss: V3,
    miss_ms: i64,
    aiming: bool,
    /// when it began aiming at its target (walking there not counted)
    aim_from: i64,
    clicked_ms: i64,
    /// walking progress: where it was at the last check, and when
    stuck_at: V3,
    stuck_ms: i64,
    stuck_count: u32,
    sidestep: Option<(i64, i8)>,
    /// it has looked for a second letter since it last placed
    looked_again: bool,
    /// the goal it walks to, the closest it has come, and when
    progress: Option<(V3, i32, i64)>,
    stalled: bool,
    /// wandering off: until when, and which way
    wander: Option<(i64, i32)>,
    /// tiles it has given up on, until the clock says
    avoid: Vec<(TileId, i64)>,
    pub thoughts: Vec<Thought>,
}

impl Brain {
    pub fn new(n: Numbers, seed: u64) -> Brain {
        Brain {
            n,
            dice: Dice(crate::rng::mix(seed ^ 0xA1C2_AB5E)),
            doing: Doing::Next,
            busy_until: 0,
            board: Board::default(),
            planned: None,
            yaw: 0,
            pitch: 0,
            started: false,
            miss: V3::ZERO,
            miss_ms: 0,
            aiming: false,
            aim_from: 0,
            clicked_ms: i64::MIN / 2,
            stuck_at: V3::ZERO,
            stuck_ms: 0,
            stuck_count: 0,
            sidestep: None,
            looked_again: false,
            progress: None,
            stalled: false,
            wander: None,
            avoid: Vec::new(),
            thoughts: Vec::new(),
        }
    }

    /// `ms` stretched or shortened by up to `jitter_pct` percent.
    fn jitter(&mut self, ms: i32) -> i64 {
        let j = self.n.jitter_pct;
        let pct = 100 + self.dice.between(-j, j);
        ms as i64 * pct as i64 / 100
    }

    fn rest(&mut self, now: i64) {
        self.doing = Doing::Next;
        self.busy_until = now + self.jitter(self.n.react_ms);
    }
}

/// What a tick of thinking asks of the crab.
#[derive(Default)]
struct Ask {
    fwd: i8,
    side: i8,
    jump: bool,
    /// where it wants to look: a heading, and a pitch (down), or `None` for
    /// level walking
    want: Option<(i32, i32)>,
    click: Option<Claw>,
    /// what to remember if the click goes
    note: Option<Thought>,
}

/// The middle of a board square's top.
pub fn square_point(m: &Match, sq: Square) -> V3 {
    let r = &m.rules;
    let at = |k: i32| -r.board_half + r.square * k + r.square / 2;
    v3(at(sq.1), r.board_top, at(sq.0))
}

/// How far a point is from the board's edge, flat (0 on or over it).
fn off_board(m: &Match, p: V3) -> i32 {
    let h = m.rules.board_half;
    let dx = (p.x.abs() - h).max(0);
    let dz = (p.z.abs() - h).max(0);
    v3(dx, 0, dz).len()
}

/// The letters of face-up loose tiles within sight of seat `seat`.
pub fn seen(m: &Match, seat: usize, see_mm: i32, avoid: &[(TileId, i64)]) -> Vec<Seen> {
    let pos = m.crabs[seat].pos;
    let range = mm1(see_mm);
    m.tiles
        .all
        .iter()
        .filter(|t| t.is_loose() && !t.face_down)
        // a tile with another lying on it shows too little of its letter
        .filter(|t| {
            !m.tiles
                .all
                .iter()
                .any(|o| o.is_loose() && o.rests_on.contains(&t.id))
        })
        .filter(|t| !avoid.iter().any(|(a, _)| *a == t.id))
        // one high up a pile, out of reach from where it stands, it leaves
        .filter(|t| t.middle().y - pos.y <= m.rules.reach_up - mm1(HIGH_MARGIN_MM))
        .filter_map(|t| {
            let d = t.pose.t.sub(pos).flat();
            (!d.longer_than(range)).then(|| Seen {
                tile: t.id,
                letter: t.letter,
                // a climb up a pile counts three times its height
                mm: ((d.len() as i64 + 3 * (t.pose.t.y - pos.y).max(0) as i64) * 1000 / fx::ONE as i64)
                    as i32,
            })
        })
        .collect()
}

/// How many different letters are in view.
fn distinct(view: &[Seen]) -> i32 {
    let mut ls: Vec<char> = view.iter().map(|s| s.letter).collect();
    ls.sort_unstable();
    ls.dedup();
    ls.len() as i32
}

/// The nearest loose tile to seat `seat`, face up or down, not avoided.
fn nearest_tile(m: &Match, seat: usize, avoid: &[(TileId, i64)]) -> Option<TileId> {
    let pos = m.crabs[seat].pos;
    m.tiles
        .all
        .iter()
        .filter(|t| t.is_loose() && !avoid.iter().any(|(a, _)| *a == t.id))
        .min_by_key(|t| (t.pose.t.sub(pos).flat().len(), t.id))
        .map(|t| t.id)
}

fn letters_held(m: &Match, seat: usize) -> [Option<char>; 2] {
    let c = &m.crabs[seat];
    [0, 1].map(|k| c.claws[k].and_then(|id| m.tiles.get(id)).map(|t| t.letter))
}

/// How common a letter is in a set (the drop picks the rarer).
fn common(l: char) -> u32 {
    crate::letters::COUNTS
        .iter()
        .find(|(c, _)| *c == l)
        .map_or(0, |(_, n)| *n)
}

/// Seat `seat`'s input for the next tick. Call it just before
/// `Match::step`, once a tick, for each AI seat.
pub fn think(m: &Match, seat: usize, b: &mut Brain) -> SeatInput {
    let c = &m.crabs[seat];
    if !b.started {
        b.started = true;
        b.yaw = c.yaw;
        b.pitch = c.pitch;
        b.stuck_at = c.pos;
    }
    let now = m.play_ms();
    let running = matches!(m.phase(), Phase::Running { ms_left } if ms_left > 0);
    if !running {
        return SeatInput {
            yaw: b.yaw,
            pitch: b.pitch,
            ..Default::default()
        };
    }
    b.avoid.retain(|(_, until)| *until > now);
    if off_board(m, c.pos) <= mm1(b.n.see_mm) {
        b.board = Board::from_tiles(&m.tiles);
    }
    let ask = step(m, seat, b, now);
    // the camera turns toward where it wants to look, at its turn rate
    let turn = (deg(b.n.turn_deg_per_s) as i64 * m.rules.tick_ms / 1000) as i32;
    let (want_yaw, want_pitch) = ask.want.unwrap_or((b.yaw, m.rules.pitch_range.0.max(deg(15))));
    b.yaw = fx::wrap(b.yaw + fx::wrap(want_yaw - b.yaw).clamp(-turn, turn));
    let (lo, hi) = m.rules.pitch_range;
    b.pitch = (b.pitch + (want_pitch.clamp(lo, hi) - b.pitch).clamp(-turn, turn)).clamp(lo, hi);
    let mut click = [false; 2];
    // a click only with the camera still, so the step judges it with the
    // crosshair it was checked on (a full claw aimed off would drop)
    let still = b.yaw == c.yaw && b.pitch == c.pitch.clamp(lo, hi) && ask.fwd == 0;
    if let Some(k) = ask.click.filter(|_| still) {
        if now - b.clicked_ms >= CLICK_WAIT_MS {
            click[k] = true;
            b.clicked_ms = now;
            b.thoughts.extend(ask.note);
        }
    }
    SeatInput {
        fwd: ask.fwd,
        side: ask.side,
        jump: ask.jump,
        yaw: b.yaw,
        pitch: b.pitch,
        click,
    }
}

/// One tick of the cycle.
fn step(m: &Match, seat: usize, b: &mut Brain, now: i64) -> Ask {
    let c = &m.crabs[seat];
    let held = letters_held(m, seat);
    let idle = now - c.placed_ms;
    let hurry = idle >= b.n.hurry_ms as i64;
    if b.stalled {
        b.stalled = false;
        b.progress = None;
        let tile = match b.doing {
            Doing::ToPile { tile } | Doing::Fetch { tile, .. } => Some(tile),
            _ => None,
        };
        // a pile it cannot get into: every tile round that one
        if let Some(t) = tile.and_then(|t| m.tiles.get(t)) {
            let until = now + 4 * b.n.give_up_ms as i64;
            for o in &m.tiles.all {
                if o.is_loose() && !o.pose.t.sub(t.pose.t).flat().longer_than(mm1(300)) {
                    b.avoid.push((o.id, until));
                }
            }
        }
        give_up(b, tile, now);
        // and it wanders off a little, a random way, jumping, to come
        // loose of what held it
        let yaw = b.dice.below(fx::TURN as u32) as i32;
        b.wander = Some((now + WANDER_MS, yaw));
        return Ask::default();
    }
    if let Some((until, yaw)) = b.wander {
        if now < until {
            let hop = (now / STUCK_MS) % 2 == 0 && now % STUCK_MS == 0;
            return Ask {
                fwd: 1,
                jump: hop,
                want: Some((yaw, m.rules.pitch_range.0.max(deg(15)))),
                ..Default::default()
            };
        }
        b.wander = None;
    }
    match b.doing.clone() {
        Doing::Next => {
            if now < b.busy_until {
                return Ask::default();
            }
            decide(m, seat, b, now, hurry);
            Ask::default()
        }
        Doing::ToPile { tile } => {
            let Some(t) = m.tiles.get(tile).filter(|t| t.is_loose()) else {
                b.rest(now);
                return Ask::default();
            };
            let goal = t.pose.t;
            if !goal.sub(c.pos).flat().longer_than(mm1(b.n.see_mm) * 2 / 3) {
                let view = seen(m, seat, b.n.see_mm, &b.avoid);
                let think =
                    b.jitter(b.n.letter_ms + b.n.per_letter_ms * distinct(&view)) + b.jitter(b.n.plan_ms);
                b.doing = Doing::Choose {
                    until: now + think,
                    hurry: false,
                };
                return Ask::default();
            }
            walk(m, seat, b, goal, 0, now)
        }
        Doing::Choose { until, hurry: rushed } => {
            if now < until {
                // looking the pile over
                return Ask::default();
            }
            let claw = if held[0].is_none() { 0 } else { 1 };
            let pick = if rushed {
                None
            } else {
                plan::pick(
                    &b.board,
                    &seen(m, seat, b.n.see_mm, &b.avoid),
                    seat,
                    b.n.steal_pct,
                    b.n.walk_cost_per_m,
                )
            };
            // a second look that found no word: to the board with the first
            if b.looked_again
                && held.iter().any(|h| h.is_some())
                && pick.as_ref().is_none_or(|p| p.plan.is_none())
            {
                decide(m, seat, b, now, false);
                return Ask::default();
            }
            let (tile, letter, planned) = match pick {
                Some(p) => {
                    b.planned = p.plan.as_ref().map(|q| q.square);
                    (Some(p.tile), Some(p.letter), p.plan.is_some())
                }
                // nothing face up in view (or in a hurry): the nearest, blind
                None => (nearest_tile(m, seat, &b.avoid), None, false),
            };
            let Some(tile) = tile else {
                b.rest(now);
                return Ask::default();
            };
            b.thoughts.push(Thought::Chose {
                ms: now,
                tile,
                letter,
                planned,
            });
            start_aim(b, now);
            b.doing = Doing::Fetch {
                tile,
                claw,
                since: now,
            };
            Ask::default()
        }
        Doing::Fetch { tile, claw, since } => {
            if c.claws[claw].is_some() {
                b.rest(now);
                return Ask::default();
            }
            if m.tiles.get(tile).is_none_or(|t| !t.is_loose()) {
                b.rest(now);
                return Ask::default();
            }
            if b.aiming && now - b.aim_from > b.n.give_up_ms as i64 {
                give_up(b, Some(tile), now);
                return Ask::default();
            }
            // the crosshair on another tile in reach, as good as this one
            // (the same letter, or this one was picked blind), or after a
            // while any face-up one with a real word to make: that one
            let aim = m.aim(seat);
            if let Target::Tile(other) = aim.target {
                let (want, got) = (m.tiles.get(tile), m.tiles.get(other));
                if let (Some(want), Some(got)) = (want, got) {
                    let same = other != tile
                        && got.is_loose()
                        && aim.in_reach
                        && (want.face_down || got.letter == want.letter);
                    let will_do = other != tile
                        && aim.in_reach
                        && got.is_loose()
                        && !got.face_down
                        && b.aiming
                        && now - b.aim_from > SWITCH_MS
                        && plan::best(&b.board, &[got.letter], seat, b.n.steal_pct, true).is_some();
                    if same || will_do {
                        b.doing = Doing::Fetch {
                            tile: other,
                            claw,
                            since,
                        };
                        return reach_and_click(m, seat, b, Target::Tile(other), claw, now);
                    }
                }
            }
            reach_and_click(m, seat, b, Target::Tile(tile), claw, now)
        }
        Doing::ToBoard { square } => {
            let goal = square_point(m, square);
            if off_board(m, c.pos) <= mm1(b.n.see_mm) / 2 || !goal.sub(c.pos).flat().longer_than(stand(m)) {
                b.doing = Doing::Settle {
                    until: now + b.jitter(b.n.place_ms),
                };
                return Ask::default();
            }
            walk(m, seat, b, goal, stand(m), now)
        }
        Doing::Settle { until } => {
            if now < until {
                return Ask::default();
            }
            settle(m, seat, b, now, hurry);
            Ask::default()
        }
        Doing::Place { claw, square, .. } => {
            let Some(id) = c.claws[claw] else {
                // placed (or lost): what next
                b.planned = None;
                b.looked_again = false;
                b.rest(now);
                return Ask::default();
            };
            let letter = m.tiles.get(id).map_or('?', |t| t.letter);
            if !b.board.legal(square) {
                // taken while it came: think again
                b.doing = Doing::Settle {
                    until: now + b.jitter(b.n.place_ms),
                };
                return Ask::default();
            }
            if b.aiming && now - b.aim_from > b.n.give_up_ms as i64 {
                give_up(b, None, now);
                return Ask::default();
            }
            let mut ask = reach_and_click(m, seat, b, Target::Square(square), claw, now);
            if ask.click.is_some() {
                let p = plan::rate(&b.board, square, letter, seat, b.n.steal_pct);
                ask.note = Some(Thought::Placing {
                    ms: now,
                    square,
                    letter,
                    real: p.real,
                    steal: p.steal,
                });
            }
            ask
        }
        Doing::Drop { claw, since } => {
            let Some(id) = c.claws[claw] else {
                b.rest(now);
                return Ask::default();
            };
            if now - since > b.n.give_up_ms as i64 {
                give_up(b, None, now);
                return Ask::default();
            }
            // look up off the board and the tiles, and let go
            let letter = m.tiles.get(id).map_or('?', |t| t.letter);
            let want = (b.yaw, m.rules.pitch_range.0);
            let settled = (b.pitch - want.1).abs() < deg(2);
            Ask {
                want: Some(want),
                click: settled.then_some(claw),
                note: settled.then_some(Thought::Dropping { ms: now, letter }),
                ..Default::default()
            }
        }
    }
}

/// How far from a square's middle it stands to place on it.
fn stand(m: &Match) -> i32 {
    m.rules.claw_tip + m.rules.reach / 2
}

fn give_up(b: &mut Brain, tile: Option<TileId>, now: i64) {
    if let Some(t) = tile {
        b.avoid.push((t, now + 4 * b.n.give_up_ms as i64));
    }
    b.planned = None;
    let what = match b.doing {
        Doing::ToPile { .. } => "to_pile",
        Doing::Fetch { .. } => "fetch",
        Doing::ToBoard { .. } => "to_board",
        Doing::Place { .. } => "place",
        Doing::Drop { .. } => "drop",
        _ => "other",
    };
    b.thoughts.push(Thought::GaveUp { ms: now, what });
    b.rest(now);
}

/// Between steps: to the board with what it holds, or to a pile.
fn decide(m: &Match, seat: usize, b: &mut Brain, now: i64, hurry: bool) {
    let held = letters_held(m, seat);
    let letters: Vec<char> = held.iter().flatten().copied().collect();
    let full = held.iter().all(|h| h.is_some());
    if !letters.is_empty() {
        let plan = plan::best(&b.board, &letters, seat, b.n.steal_pct, !hurry);
        if let Some((_, p)) = &plan {
            // one claw free and time to spare at a pile: a look for a
            // second letter first
            let spare = now - m.crabs[seat].placed_ms < b.n.hurry_ms as i64 - SECOND_LOOK_SPARE_MS;
            let view = seen(m, seat, b.n.see_mm, &b.avoid);
            if !full && !b.looked_again && spare && !view.is_empty() {
                b.looked_again = true;
                let think = b.jitter(b.n.letter_ms + b.n.per_letter_ms * distinct(&view));
                b.doing = Doing::Choose {
                    until: now + think,
                    hurry: false,
                };
                return;
            }
            let square = b.planned.filter(|s| b.board.legal(*s)).unwrap_or(p.square);
            b.doing = Doing::ToBoard { square };
            return;
        }
        if hurry || full {
            // nothing real to make: in a hurry, the board anyway (it
            // places the least bad there); with both claws full, it lets
            // the rarer letter go
            if hurry {
                let square = b.planned.unwrap_or((plan::N / 2, plan::N / 2));
                b.doing = Doing::ToBoard { square };
            } else {
                let claw = if common(held[0].unwrap_or('?')) < common(held[1].unwrap_or('?')) {
                    0
                } else {
                    1
                };
                b.doing = Doing::Drop { claw, since: now };
            }
            return;
        }
    }
    if hurry {
        // empty-handed and late: the nearest tile, no browsing
        b.doing = Doing::Choose {
            until: now,
            hurry: true,
        };
        return;
    }
    match pile(m, seat, &b.avoid) {
        Some(tile) => b.doing = Doing::ToPile { tile },
        None => b.rest(now),
    }
}

/// The pile to walk to: the loose tile with the most loose tiles round it
/// for its distance (a metre is worth four tiles).
fn pile(m: &Match, seat: usize, avoid: &[(TileId, i64)]) -> Option<TileId> {
    let pos = m.crabs[seat].pos;
    let loose: Vec<_> = m
        .tiles
        .all
        .iter()
        .filter(|t| t.is_loose() && !avoid.iter().any(|(a, _)| *a == t.id))
        .collect();
    let near = mm1(400);
    loose
        .iter()
        .min_by_key(|t| {
            let d = t.pose.t.sub(pos).flat().len() as i64;
            let round = loose
                .iter()
                .filter(|o| !o.pose.t.sub(t.pose.t).flat().longer_than(near))
                .count() as i64;
            (d - round * fx::ONE as i64 / 4, t.id)
        })
        .map(|t| t.id)
}

/// At the board, the thinking done: place, fetch another, or drop.
fn settle(m: &Match, seat: usize, b: &mut Brain, now: i64, hurry: bool) {
    let held = letters_held(m, seat);
    let claws: Vec<Claw> = (0..2).filter(|k| held[*k].is_some()).collect();
    let letters: Vec<char> = claws.iter().map(|k| held[*k].unwrap_or('?')).collect();
    match plan::best(&b.board, &letters, seat, b.n.steal_pct, !hurry) {
        Some((k, p)) => {
            start_aim(b, now);
            b.doing = Doing::Place {
                claw: claws[k],
                square: p.square,
                since: now,
            };
        }
        None => {
            b.planned = None;
            if claws.len() == 2 {
                decide(m, seat, b, now, hurry);
            } else {
                b.rest(now);
            }
        }
    }
}

/// A new target: its aim's miss is rolled when it first aims at it.
fn start_aim(b: &mut Brain, _now: i64) {
    b.aiming = false;
}

/// Walks to within reach of `goal`, aims the crosshair at it and clicks
/// `claw` once a click there would do what it means: grab that tile, or
/// place on that square (`Match::click`, the crosshair a person sees).
/// Aimed right but out of reach, it shuffles closer.
fn reach_and_click(m: &Match, seat: usize, b: &mut Brain, goal: Target, claw: Claw, now: i64) -> Ask {
    let c = &m.crabs[seat];
    let r = &m.rules;
    let target = match goal {
        Target::Tile(id) => match m.tiles.get(id) {
            // the top face's middle, which a look from above meets first
            Some(t) => t.pose.apply(v3(0, mm1(THICK_MM), 0)),
            None => return Ask::default(),
        },
        Target::Square(sq) => square_point(m, sq),
        Target::Nothing => return Ask::default(),
    };
    if target.sub(c.pos).flat().longer_than(stand(m) + mm1(30)) {
        return walk(m, seat, b, target, stand(m), now);
    }
    // up a pile, out of reach above it: it climbs on toward it (jumping
    // when stuck, giving up when that gets it nowhere)
    if target.y - c.pos.y > r.reach_up {
        return walk(m, seat, b, target, 0, now);
    }
    // the aim starts off by up to `aim_error_mm` and the miss halves as
    // it corrects
    if !b.aiming {
        b.aiming = true;
        let e = b.n.aim_error_mm;
        b.miss = v3(mm1(b.dice.between(-e, e)), 0, mm1(b.dice.between(-e, e)));
        b.miss_ms = now;
        b.aim_from = now;
    }
    while now - b.miss_ms >= b.n.aim_settle_ms as i64 {
        b.miss = v3(b.miss.x / 2, 0, b.miss.z / 2);
        b.miss_ms += b.n.aim_settle_ms as i64;
    }
    let pivot = c.pos.add(v3(0, r.cam_pivot_up, 0));
    let d = target.add(b.miss).sub(pivot);
    let want = (yaw_of(d), -pitch_of(d));
    let on = match (m.click(seat, claw), goal) {
        (Some(Claim::Grab { tile, .. }), Target::Tile(id)) => tile == id,
        (Some(Claim::Place { square, .. }), Target::Square(sq)) => square == sq,
        _ => false,
    };
    let aim = m.aim(seat);
    let closer = !on && aim.target == goal && !aim.in_reach;
    Ask {
        want: Some(want),
        click: on.then_some(claw),
        fwd: closer as i8,
        ..Default::default()
    }
}

/// Walks toward `goal` until within `stop` of it (flat): faces its way,
/// W when facing near enough, a jump when stuck, then a step aside.
fn walk(m: &Match, seat: usize, b: &mut Brain, goal: V3, stop: i32, now: i64) -> Ask {
    let c = &m.crabs[seat];
    let way = goal.sub(c.pos).flat();
    if !way.longer_than(stop) {
        return Ask::default();
    }
    // no closer by `PROGRESS_MM` in `STALL_MS`: it gives this goal up
    let dist = way.len();
    match b.progress {
        Some((g, best, since)) if !g.sub(goal).flat().longer_than(mm1(50)) => {
            if dist < best - mm1(PROGRESS_MM) {
                b.progress = Some((goal, dist, now));
            } else if now - since > STALL_MS {
                b.stalled = true;
            }
        }
        _ => b.progress = Some((goal, dist, now)),
    }
    let heading = yaw_of(way);
    let level = m.rules.pitch_range.0.max(deg(15));
    let mut ask = Ask {
        want: Some((heading, level)),
        ..Default::default()
    };
    if let Some((until, side)) = b.sidestep {
        if now < until {
            ask.side = side;
            ask.fwd = 1;
            return ask;
        }
        b.sidestep = None;
    }
    if fx::wrap(heading - b.yaw).abs() < WALK_ARC {
        ask.fwd = 1;
    }
    // progress
    if now - b.stuck_ms >= STUCK_MS {
        let should = (m.rules.speed as i64 * STUCK_MS / 1000 / 4) as i32;
        let gone = c.pos.sub(b.stuck_at).flat();
        if ask.fwd != 0 && !gone.longer_than(should) {
            b.stuck_count += 1;
            if b.stuck_count % 2 == 1 {
                ask.jump = true;
            } else {
                let side = if b.dice.below(2) == 0 { -1 } else { 1 };
                b.sidestep = Some((now + SIDESTEP_MS, side));
            }
        } else {
            b.stuck_count = 0;
        }
        b.stuck_at = c.pos;
        b.stuck_ms = now;
    }
    ask
}

/// Whether a tile is held by seat `seat` (for tests and tools).
pub fn holds(m: &Match, seat: usize, tile: TileId) -> bool {
    matches!(m.tiles.get(tile).map(|t| t.state), Some(State::Held { seat: s, .. }) if s == seat)
}

#[cfg(test)]
mod tests;
