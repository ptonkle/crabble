//! The stand-in game, until the real one is built: crabs on flat sand
//! around the board's outline, the local crab walked with W A S D and
//! turned with the mouse, a 3-2-1 countdown, then a game clock counting down
//! from `game.minutes` (`data/tuning.json`) to the end. No tiles, no words,
//! no score: the menus, the lobby and the online sync have something to
//! start, steer and end.
//!
//! The AI crabs only wander to random points (`tuning.wander`); the real AI
//! comes with the real game. Online, crabs another browser owns are placed
//! from their states (`set_remote`).

pub mod draw;
pub mod trig;

use serde::Serialize;

use crate::online::CrabState;
use crate::rng::Dice;
use crate::tuning::Tuning;
use trig::{atan2, cos, isqrt, sin, TURN};

/// The longest step the crabs move in one go, in milliseconds: a slow frame
/// moves them in several steps.
const MAX_STEP_MS: i64 = 50;

/// Who moves a crab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Driver {
    /// this browser's player
    Local,
    /// the stand-in AI, on this browser
    Ai,
    /// another browser, through its states
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crab {
    pub driver: Driver,
    /// where it stands on the sand, millimetres from the board's centre
    /// (x east, z north)
    pub x: i32,
    pub z: i32,
    /// millimetres a second
    pub vx: i32,
    pub vz: i32,
    /// which way it faces, angle units (0 is north, a quarter turn east)
    pub yaw: i32,
    /// the AI's next point
    target: (i32, i32),
}

impl Crab {
    pub fn state(&self) -> CrabState {
        CrabState {
            p: [self.x, self.z],
            v: [self.vx, self.vz],
            y: self.yaw,
        }
    }
}

/// The movement keys held.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Keys {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
}

impl Keys {
    /// Sets the key `code` (`KeyboardEvent.code`) held or let go; false
    /// for a key that does not move the crab.
    pub fn set(&mut self, code: &str, down: bool) -> bool {
        let k = match code {
            "KeyW" => &mut self.forward,
            "KeyS" => &mut self.back,
            "KeyA" => &mut self.left,
            "KeyD" => &mut self.right,
            _ => return false,
        };
        *k = down;
        true
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    pub crabs: Vec<Crab>,
    /// the crab this browser steers
    pub local: usize,
    /// the camera's turn and tilt (angle units; the tilt looks down)
    pub yaw: i32,
    pub pitch: i32,
    pub keys: Keys,
    /// the game's own clock: milliseconds since its countdown began
    pub elapsed_ms: i64,
    countdown_ms: i64,
    length_ms: i64,
    /// the clock stops at 0:00 and waits for `finish` (an online joiner
    /// waits for the host's End)
    pub hold_at_zero: bool,
    finished: bool,
    dice: Dice,
    /// how far from the centre a crab may go, each way
    pub extent_mm: i32,
    speed: i32,
    arrive: i32,
    turn_per_px: i32,
    pitch_range: (i32, i32),
}

/// A game's setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    /// each crab's driver, in seat (and colour) order
    pub drivers: Vec<Driver>,
    pub local: usize,
    pub seed: u64,
    /// `?fast`: no countdown (`app/fast.rs`)
    pub fast: bool,
    /// the game's length, when not `game.minutes` (the browser checks'
    /// `?short`)
    pub length_ms: Option<i64>,
}

/// Each corner's start, in seat order: red, yellow, blue, purple clockwise
/// from the south-west corner (SECOND-ORDER R1.14), as (x sign, z sign, the
/// yaw that faces the centre).
const CORNERS: [(i32, i32, i32); 4] = [(-1, -1, 8192), (-1, 1, 24576), (1, 1, 40960), (1, -1, 57344)];

impl Game {
    pub fn new(t: &Tuning, setup: Setup) -> Game {
        let half = t.board_mm() / 2;
        // along the diagonal: off by start_off_corner_mm on each axis / sqrt 2
        let off = (t.crab.start_off_corner_mm as i64 * 46341 / 65536) as i32;
        let crabs: Vec<Crab> = setup
            .drivers
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let (sx, sz, yaw) = CORNERS[i % 4];
                let (x, z) = (sx * (half + off), sz * (half + off));
                Crab {
                    driver: *d,
                    x,
                    z,
                    vx: 0,
                    vz: 0,
                    yaw,
                    target: (x, z),
                }
            })
            .collect();
        let yaw = crabs.get(setup.local).map_or(0, |c| c.yaw);
        Game {
            crabs,
            local: setup.local,
            yaw,
            pitch: t.look.pitch_start,
            keys: Keys::default(),
            elapsed_ms: 0,
            countdown_ms: if setup.fast {
                0
            } else {
                t.game.countdown_seconds as i64 * 1000
            },
            length_ms: setup.length_ms.unwrap_or_else(|| t.game_ms()),
            hold_at_zero: false,
            finished: false,
            dice: Dice(setup.seed),
            extent_mm: half + t.crab.roam_mm,
            speed: t.crab.speed_mm_per_s,
            arrive: t.wander.arrive_mm,
            turn_per_px: t.look.turn_per_px,
            pitch_range: (t.look.pitch_min, t.look.pitch_max),
        }
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
        self.finished = true;
    }

    /// The clock has run out (the end, or a joiner waiting at 0:00).
    pub fn out_of_time(&self) -> bool {
        self.finished || self.elapsed_ms >= self.countdown_ms + self.length_ms
    }

    /// Moves the clock on by `ms`, and while the game runs, the crabs this
    /// browser moves.
    pub fn advance(&mut self, ms: i64) {
        let mut left = ms.max(0);
        while left > 0 {
            let step = left.min(MAX_STEP_MS);
            left -= step;
            self.elapsed_ms += step;
            if matches!(self.phase(), Phase::Running { ms_left } if ms_left > 0) {
                self.step(step);
            } else {
                for c in &mut self.crabs {
                    if c.driver != Driver::Remote {
                        c.vx = 0;
                        c.vz = 0;
                    }
                }
            }
        }
    }

    fn step(&mut self, ms: i64) {
        let keys = self.keys;
        let yaw = self.yaw;
        let (speed, extent, arrive) = (self.speed, self.extent_mm, self.arrive);
        for i in 0..self.crabs.len() {
            match self.crabs[i].driver {
                Driver::Local => {
                    let c = &mut self.crabs[i];
                    c.yaw = yaw;
                    let (f, r) = (
                        keys.forward as i32 - keys.back as i32,
                        keys.right as i32 - keys.left as i32,
                    );
                    // forward is (sin, cos), right is (cos, -sin)
                    let (s, co) = (sin(yaw) as i64, cos(yaw) as i64);
                    let mut wx = f as i64 * s + r as i64 * co;
                    let mut wz = f as i64 * co - r as i64 * s;
                    if f != 0 && r != 0 {
                        // a diagonal is no faster
                        wx = wx * 46341 / 65536;
                        wz = wz * 46341 / 65536;
                    }
                    c.vx = (wx * speed as i64 / 65536) as i32;
                    c.vz = (wz * speed as i64 / 65536) as i32;
                }
                Driver::Ai => {
                    let c = &self.crabs[i];
                    let (dx, dz) = ((c.target.0 - c.x) as i64, (c.target.1 - c.z) as i64);
                    let dist = isqrt(dx * dx + dz * dz);
                    if dist <= arrive as i64 {
                        let t = (
                            self.dice.between(-extent, extent),
                            self.dice.between(-extent, extent),
                        );
                        self.crabs[i].target = t;
                        continue;
                    }
                    let c = &mut self.crabs[i];
                    c.yaw = atan2(dx, dz);
                    c.vx = (dx * speed as i64 / dist) as i32;
                    c.vz = (dz * speed as i64 / dist) as i32;
                }
                Driver::Remote => continue,
            }
            let c = &mut self.crabs[i];
            c.x = (c.x + (c.vx as i64 * ms / 1000) as i32).clamp(-extent, extent);
            c.z = (c.z + (c.vz as i64 * ms / 1000) as i32).clamp(-extent, extent);
        }
    }

    /// The mouse moved by (dx, dy) pixels: the camera turns, at
    /// `sensitivity_pct` percent, and tilts (the other way with `invert_y`).
    pub fn look(&mut self, dx: i32, dy: i32, sensitivity_pct: u32, invert_y: bool) {
        let per = self.turn_per_px as i64 * sensitivity_pct as i64;
        self.yaw = (self.yaw as i64 + dx as i64 * per / 100).rem_euclid(TURN as i64) as i32;
        let dy = if invert_y { -dy } else { dy };
        let (lo, hi) = self.pitch_range;
        self.pitch = ((self.pitch as i64 + dy as i64 * per / 100) as i32).clamp(lo, hi);
    }

    /// Crab `i` as its owner last said, for another browser's crab.
    pub fn set_remote(&mut self, i: usize, s: CrabState) {
        if let Some(c) = self.crabs.get_mut(i) {
            if c.driver == Driver::Remote {
                c.x = s.p[0];
                c.z = s.p[1];
                c.vx = s.v[0];
                c.vz = s.v[1];
                c.yaw = s.y;
            }
        }
    }
}

#[cfg(test)]
mod tests;
