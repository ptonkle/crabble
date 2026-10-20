//! The game itself: one [`Match`] holds everything a game is (PLAN.md,
//! gameplay): the ground, the board, every tile, every crab, the clock and
//! the events. [`Match::step`] moves it one tick of `game.tick_ms` from
//! one [`SeatInput`] per seat. The same seed, map and inputs give the same
//! game on every machine (no floats, no clock, no randomness of its own).
//!
//! People and AI crabs both drive their crabs through `SeatInput`; a crab
//! another browser moves (online) takes no input and is placed from its
//! owner's states ([`Match::set_remote`]).
//!
//! On screen this is a *game* (GLOSSARY.md); `Match` is the person's name
//! for the simulation (2026-10-19).

pub mod crab;
pub mod ground;
pub mod map;
pub mod tiles;

use serde::Serialize;

use crate::gfx::geom::{v3, V3};
use crate::letters;
use crate::model::mm1;
use crate::online::CrabState;
use crate::rng::Dice;
use crate::tuning::Tuning;

pub use crab::{Crab, Driver, Gait};
pub use map::Map;
pub use tiles::{Claw, Square, State, Tile, TileId, Tiles};

use ground::Ground;

/// How many standard sets of tiles a game deals (BRIEF.md, Tiles).
pub const SETS: u32 = 3;

/// One seat's input for one tick: what a person's keys and mouse, or an
/// AI crab's choices, ask of its crab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeatInput {
    /// W minus S, and D minus A: -1, 0 or 1
    pub fwd: i8,
    pub side: i8,
    /// Space pressed since the last tick
    pub jump: bool,
    /// where the seat's camera looks (angle units; the pitch looks down)
    pub yaw: i32,
    pub pitch: i32,
    /// each claw's button pressed since the last tick: left, right
    pub click: [bool; 2],
}

/// Something that happened, for the sounds, the popups and (later) the AI
/// and the online messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    Jumped { seat: usize },
    Landed { seat: usize },
    Ended,
}

/// Where the game is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Phase {
    /// the 3-2-1: whole seconds left, rounded up
    Countdown {
        seconds: u32,
    },
    /// the game clock, in milliseconds left
    Running {
        ms_left: i64,
    },
    Ended,
}

/// A game's setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    /// each crab's driver, in seat (and colour) order
    pub drivers: Vec<Driver>,
    pub seed: u64,
    /// `?fast`: no countdown (`app/fast.rs`)
    pub fast: bool,
    /// the game's length, when not `game.minutes` (the browser checks'
    /// `?short`)
    pub length_ms: Option<i64>,
}

/// Each corner's start, in seat order: red, yellow, blue, purple round
/// the board (SECOND-ORDER R1.14), as the signs of x and z.
const CORNERS: [(i32, i32); 4] = [(-1, 1), (-1, -1), (1, -1), (1, 1)];

/// The rule numbers a game runs by, from the tuning, in the core's units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    pub tick_ms: i64,
    /// Q16 metres a second
    pub speed: i32,
    pub jump_speed: i32,
    /// Q16 metres a second squared
    pub gravity: i32,
    pub fall_max: i32,
    /// Q16 metres
    pub step: i32,
    pub climb: i32,
    pub foot: i32,
    pub radius: i32,
    pub push_pct: i32,
    /// how far a crab may go from the middle, each way
    pub extent: i32,
    pub pitch_range: (i32, i32),
    /// the camera: how far back, the point it looks at above the feet,
    /// and the least it stays over the sand
    pub cam_distance: i32,
    pub cam_pivot_up: i32,
    pub cam_clear: i32,
    /// how far ahead the claw tips are, and how far past them a crab
    /// reaches
    pub claw_tip: i32,
    pub reach: i32,
    /// the board: half its squares' side, a square, the height of its top
    pub board_half: i32,
    pub square: i32,
    pub board_size: i32,
    pub board_top: i32,
}

impl Rules {
    pub fn new(t: &Tuning) -> Rules {
        let (jump_speed, gravity) = t.jump();
        Rules {
            tick_ms: t.game.tick_ms as i64,
            speed: mm1(t.crab.speed_mm_per_s),
            jump_speed,
            gravity,
            fall_max: mm1(t.crab.fall_max_mm_per_s),
            step: mm1(t.crab.step_mm),
            climb: mm1(t.crab.climb_mm),
            foot: mm1(t.crab.foot_mm),
            radius: mm1(t.crab.radius_mm),
            push_pct: t.crab.push_pct,
            extent: mm1(t.board_mm() / 2 + t.crab.roam_mm),
            pitch_range: (t.look.pitch_min, t.look.pitch_max),
            cam_distance: mm1(t.camera.distance_mm),
            cam_pivot_up: mm1(t.camera.pivot_up_mm),
            cam_clear: mm1(t.camera.clear_mm),
            claw_tip: mm1(t.crab.claw_tip_mm),
            reach: mm1(t.crab.reach_mm),
            board_half: mm1(t.board_mm() / 2),
            square: mm1(t.board.square_mm),
            board_size: t.board.size as i32,
            board_top: mm1(t.board.top_mm),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub rules: Rules,
    pub crabs: Vec<Crab>,
    pub tiles: Tiles,
    pub ground: Ground,
    /// the game's own clock: milliseconds since its countdown began
    pub elapsed_ms: i64,
    countdown_ms: i64,
    length_ms: i64,
    /// the clock stops at 0:00 and waits for `finish` (an online joiner
    /// waits for the host's End)
    pub hold_at_zero: bool,
    finished: bool,
    /// what happened since the app last took them
    pub events: Vec<Event>,
    pub dice: Dice,
}

/// The start of each seat, in seat order: just off its corner of the
/// board, facing the middle.
pub fn starts(t: &Tuning, seats: usize) -> Vec<(V3, i32)> {
    let half = t.board_mm() / 2;
    // along the diagonal: start_off_corner_mm / sqrt 2 on each axis
    let off = (t.crab.start_off_corner_mm as i64 * 46341 / 65536) as i32;
    (0..seats)
        .map(|i| {
            let (sx, sz) = CORNERS[i % 4];
            let at = v3(mm1(sx * (half + off)), 0, mm1(sz * (half + off)));
            (at, crate::gfx::geom::yaw_of(at.neg()))
        })
        .collect()
}

impl Match {
    /// A game on `map`: the crabs at their corners, the tiles dealt to the
    /// map's slots.
    pub fn new(t: &Tuning, map: &Map, setup: Setup) -> Match {
        let rules = Rules::new(t);
        let mut dice = Dice(setup.seed);
        let crabs = starts(t, setup.drivers.len())
            .into_iter()
            .zip(&setup.drivers)
            .map(|((at, yaw), d)| Crab::new(*d, at, yaw, t.look.pitch_start))
            .collect();
        let mut letters = letters::tile_set(SETS);
        shuffle(&mut letters, &mut dice);
        let tiles = Tiles {
            all: map
                .slots
                .iter()
                .zip(letters)
                .enumerate()
                .map(|(i, (pose, letter))| Tile {
                    id: i as TileId,
                    letter,
                    value: letters::value(letter).unwrap_or(0),
                    state: State::Loose { awake: false },
                    pose: *pose,
                    vy: 0,
                    rests_on: Vec::new(),
                    awake_ticks: 0,
                    since_ms: 0,
                })
                .collect(),
        };
        let mut m = Match {
            rules,
            crabs,
            tiles,
            ground: map.ground.clone(),
            elapsed_ms: 0,
            countdown_ms: if setup.fast {
                0
            } else {
                t.game.countdown_seconds as i64 * 1000
            },
            length_ms: setup.length_ms.unwrap_or_else(|| t.game_ms()),
            hold_at_zero: false,
            finished: false,
            events: Vec::new(),
            dice,
        };
        for i in 0..m.crabs.len() {
            let p = m.crabs[i].pos;
            m.crabs[i].pos.y = m.support(p, None);
        }
        m
    }

    pub fn phase(&self) -> Phase {
        if self.finished {
            return Phase::Ended;
        }
        if self.elapsed_ms < self.countdown_ms {
            let left = self.countdown_ms - self.elapsed_ms;
            return Phase::Countdown {
                seconds: ((left + 999) / 1000) as u32,
            };
        }
        let left = self.length_ms - (self.elapsed_ms - self.countdown_ms);
        if left <= 0 && !self.hold_at_zero {
            return Phase::Ended;
        }
        Phase::Running { ms_left: left.max(0) }
    }

    /// The game clock's length, in milliseconds.
    pub fn length_ms(&self) -> i64 {
        self.length_ms
    }

    /// The game ends now (an online joiner: the host's End arrived).
    pub fn finish(&mut self) {
        if !self.finished {
            self.finished = true;
            self.events.push(Event::Ended);
        }
    }

    /// The clock has run out (the end, or a joiner waiting at 0:00).
    pub fn out_of_time(&self) -> bool {
        self.finished || self.elapsed_ms >= self.countdown_ms + self.length_ms
    }

    /// The game's clock since the 3-2-1 ended (ms), or 0 before.
    pub fn play_ms(&self) -> i64 {
        (self.elapsed_ms - self.countdown_ms).max(0)
    }

    /// One tick: the clock moves on `rules.tick_ms`, and while the game
    /// runs every crab this browser moves takes its seat's input (a seat
    /// with no input stands still).
    pub fn step(&mut self, inputs: &[SeatInput]) {
        let was_running = matches!(self.phase(), Phase::Running { ms_left } if ms_left > 0);
        self.elapsed_ms += self.rules.tick_ms;
        if !was_running {
            for c in &mut self.crabs {
                if c.driver != Driver::Remote {
                    c.vel = V3::ZERO;
                    c.gait = Gait::Still;
                }
            }
            return;
        }
        for i in 0..self.crabs.len() {
            if self.crabs[i].driver == Driver::Remote {
                continue;
            }
            let input = inputs.get(i).copied().unwrap_or(SeatInput {
                yaw: self.crabs[i].yaw,
                pitch: self.crabs[i].pitch,
                ..Default::default()
            });
            self.move_crab(i, &input);
        }
        self.push_crabs();
        if self.phase() == Phase::Ended && !self.finished {
            self.events.push(Event::Ended);
        }
    }

    /// The highest ground or resting tile under a crab's feet at `at`
    /// (its middle and four points `rules.foot` round it), leaving out
    /// tile `skip`.
    pub fn support(&self, at: V3, skip: Option<TileId>) -> i32 {
        self.feet(at)
            .iter()
            .map(|p| {
                let g = self.ground.height(p.x, p.z);
                self.tiles.top_at(p.x, p.z, skip).map_or(g, |t| t.max(g))
            })
            .max()
            .unwrap_or(0)
    }

    /// Whether a crab whose feet are at height `y` can walk to `at`: the
    /// drawn ground's edges under its feet (unblended) rise at most
    /// `rules.climb` above it (it scrambles onto the board and rocks), and
    /// a tile's top at most `rules.step` (it jumps onto a tile).
    fn can_walk(&self, at: V3, y: i32) -> bool {
        self.feet(at).iter().all(|p| {
            self.ground.cell_height(p.x, p.z) <= y + self.rules.climb
                && self
                    .tiles
                    .top_at(p.x, p.z, None)
                    .is_none_or(|t| t <= y + self.rules.step)
        })
    }

    fn feet(&self, at: V3) -> [V3; 5] {
        let f = self.rules.foot;
        [
            at,
            at.add(v3(f, 0, 0)),
            at.add(v3(-f, 0, 0)),
            at.add(v3(0, 0, f)),
            at.add(v3(0, 0, -f)),
        ]
    }

    fn move_crab(&mut self, i: usize, input: &SeatInput) {
        let r = self.rules.clone();
        let dt = r.tick_ms as i32;
        let per_tick = |v: i32| (v as i64 * dt as i64 / 1000) as i32;
        let (lo, hi) = r.pitch_range;
        {
            let c = &mut self.crabs[i];
            c.yaw = input.yaw;
            c.pitch = input.pitch.clamp(lo, hi);
        }
        let (fwd, side) = (input.fwd.signum() as i32, input.side.signum() as i32);
        let walk = crab::walk_velocity(input.yaw, fwd, side, r.speed);
        let c = self.crabs[i].clone();
        // across: blocked by a step too high, sliding along one axis if
        // the other is clear
        let mut pos = c.pos;
        let tries = [
            v3(per_tick(walk.x), 0, per_tick(walk.z)),
            v3(per_tick(walk.x), 0, 0),
            v3(0, 0, per_tick(walk.z)),
        ];
        let mut moved = V3::ZERO;
        if walk != V3::ZERO {
            for d in tries {
                if d == V3::ZERO {
                    continue;
                }
                let to = pos.add(d);
                let to = v3(
                    to.x.clamp(-r.extent, r.extent),
                    to.y,
                    to.z.clamp(-r.extent, r.extent),
                );
                if self.can_walk(to, c.pos.y) {
                    moved = to.sub(pos);
                    pos = to;
                    break;
                }
            }
        }
        // up and down
        let mut vy = c.vel.y;
        let mut on_ground = c.on_ground;
        let mut jumped = false;
        if on_ground && input.jump {
            vy = r.jump_speed;
            on_ground = false;
            jumped = true;
        }
        let below = self.support(pos, None);
        if on_ground {
            if below < pos.y - r.step {
                // walked off an edge
                on_ground = false;
                vy = 0;
            } else {
                pos.y = below;
                vy = 0;
            }
        }
        let mut landed = false;
        if !on_ground {
            // the mean of the speeds before and after: exact under an even
            // pull, so the apex is the tuning's
            let before = vy;
            vy = (vy - per_tick(r.gravity)).max(-r.fall_max);
            pos.y += per_tick((before + vy) / 2);
            if pos.y <= below && vy <= 0 {
                pos.y = below;
                vy = 0;
                on_ground = true;
                landed = true;
            }
        }
        let c = &mut self.crabs[i];
        c.vel = v3(
            (moved.x as i64 * 1000 / dt as i64) as i32,
            vy,
            (moved.z as i64 * 1000 / dt as i64) as i32,
        );
        c.pos = pos;
        c.on_ground = on_ground;
        c.gait = if moved == V3::ZERO {
            Gait::Still
        } else {
            crab::gait(fwd, side)
        };
        if c.gait != Gait::Still {
            c.walk_ms = c.walk_ms.wrapping_add(dt as u32);
        }
        c.air_ms = if on_ground { 0 } else { c.air_ms + dt as u32 };
        if jumped {
            self.events.push(Event::Jumped { seat: i });
        }
        if landed {
            self.events.push(Event::Landed { seat: i });
        }
    }

    /// Crabs that bump push each other apart a little (BRIEF.md: "push
    /// each other a bit"). A crab another browser moves is not pushed
    /// here; its own browser pushes it.
    fn push_crabs(&mut self) {
        let r = &self.rules;
        let n = self.crabs.len();
        for a in 0..n {
            for b in a + 1..n {
                let (pa, pb) = (self.crabs[a].pos, self.crabs[b].pos);
                if (pa.y - pb.y).abs() > r.radius * 2 {
                    continue;
                }
                let Some(d) = crab::push(pa, pb, r.radius, r.push_pct) else {
                    continue;
                };
                let (ea, eb) = (r.extent, r.extent);
                if self.crabs[a].driver != Driver::Remote {
                    let p = self.crabs[a].pos.sub(d);
                    self.crabs[a].pos = v3(p.x.clamp(-ea, ea), p.y, p.z.clamp(-ea, ea));
                }
                if self.crabs[b].driver != Driver::Remote {
                    let p = self.crabs[b].pos.add(d);
                    self.crabs[b].pos = v3(p.x.clamp(-eb, eb), p.y, p.z.clamp(-eb, eb));
                }
            }
        }
    }

    /// Crab `i` as its owner last said, for another browser's crab: it
    /// stands on the ground there.
    pub fn set_remote(&mut self, i: usize, s: CrabState) {
        let Some(c) = self.crabs.get(i) else {
            return;
        };
        if c.driver != Driver::Remote {
            return;
        }
        let pos = v3(mm1(s.p[0]), 0, mm1(s.p[1]));
        let y = self.support(pos, None);
        let c = &mut self.crabs[i];
        let moving = s.v != [0, 0];
        c.pos = v3(pos.x, y, pos.z);
        c.vel = v3(mm1(s.v[0]), 0, mm1(s.v[1]));
        c.yaw = s.y;
        c.gait = if moving { Gait::Forward } else { Gait::Still };
        if moving {
            c.walk_ms = c.walk_ms.wrapping_add(self.rules.tick_ms as u32);
        }
    }

    /// The third-person camera for a crab standing at `pos`, looking
    /// `yaw` and `pitch` (down): its eye, and the way it looks, whose
    /// centre is the crosshair. It never goes under the sand.
    pub fn camera_at(&self, pos: V3, yaw: i32, pitch: i32) -> (V3, V3) {
        let r = &self.rules;
        let pivot = pos.add(v3(0, r.cam_pivot_up, 0));
        let fwd = crate::gfx::geom::facing(yaw, -pitch);
        let mut eye = pivot.sub(fwd.scale(r.cam_distance));
        let floor = self.ground.height(eye.x, eye.z) + r.cam_clear;
        if eye.y < floor {
            eye.y = floor;
        }
        (eye, pivot.sub(eye).norm())
    }

    /// Seat `i`'s camera, as of its last tick.
    pub fn camera(&self, i: usize) -> (V3, V3) {
        let c = &self.crabs[i];
        self.camera_at(c.pos, c.yaw, c.pitch)
    }

    /// Takes what happened since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}

/// Shuffles `v` with `dice` (Fisher-Yates).
fn shuffle<T>(v: &mut [T], dice: &mut Dice) {
    for i in (1..v.len()).rev() {
        let j = dice.below(i as u32 + 1) as usize;
        v.swap(i, j);
    }
}

#[cfg(test)]
mod tests;
