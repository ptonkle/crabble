//! The tiles: every one has an id, fixed for the game, and one state
//! (PLAN.md, gameplay 5). A loose tile has a pose; a held one sits in a
//! crab's claw; a placed one is on a board square for good.
//!
//! A tile is a light rigid box, [`SIZE_MM`] square and [`THICK_MM`] thick.
//! Its `pose` is the box's frame (Q16 metres): the middle of the face it
//! lies on is the origin, the box fills y 0 to [`THICK_MM`] in it. A tile
//! face down is the same box with its letter on the underside
//! (`face_down`), drawn turned over inside it ([`Tile::draw_pose`]).
//! Its top, wherever it lies, is ground a crab can stand on.
//!
//! # Tumbling (PLAN.md, gameplay 9)
//!
//! A tile at rest sleeps. It wakes when a tile it lay on moves (is grabbed,
//! or wakes itself), or when it is dropped. Awake, it falls; when its
//! underside meets the ground or a tile under it, it settles on the plane
//! fitted to what is under its four edges, tipped no steeper than
//! `tiles.steepest_deg`. Steeper than `tiles.repose_deg` it slides
//! downhill and settles again; otherwise it sleeps. A tile awake for
//! `tiles.settle_ms` sleeps wherever it is. All integer: the same on every
//! machine. Only the referee runs it (online, the host).

use crate::gfx::fx::{self, ONE};
use crate::gfx::geom::{v3, Affine, V3};
use crate::model::mm1;

use super::ground::Ground;

pub type TileId = u16;

/// A tile's side and thickness (`data/models/tile.json`).
pub const SIZE_MM: i32 = 200;
pub const THICK_MM: i32 = 45;

/// A claw: 0 the left, 1 the right.
pub type Claw = usize;
pub const LEFT: Claw = 0;
pub const RIGHT: Claw = 1;

/// A board square, (row, column), both from 0.
pub type Square = (i32, i32);

/// How far under a point of a tile's underside another tile's top may lie
/// and still hold it up, and how far over it a top may poke and still
/// count as under it.
const REST_GAP_MM: i32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// on the sand, a mound or a pile; `awake` while it tumbles
    Loose { awake: bool },
    /// in a crab's claw
    Held { seat: usize, claw: Claw },
    /// on the board for good: by whom (none for the centre tile), and the
    /// how-manyth placement of the game it was (0 for the centre tile)
    Placed {
        square: Square,
        seat: Option<usize>,
        order: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    pub id: TileId,
    pub letter: char,
    pub value: i32,
    pub state: State,
    /// the box's frame where it lies (loose or placed); a held tile keeps
    /// where it was until it is let go
    pub pose: Affine,
    pub face_down: bool,
    /// falling speed, Q16 metres a second (down is negative), and its
    /// slide downhill (flat, Q16 metres a second)
    pub vy: i32,
    pub slide: V3,
    /// the tiles it lay on when it last came to rest: when one of them
    /// moves, this one wakes
    pub rests_on: Vec<TileId>,
    /// ticks it has been awake, so a tile that cannot settle still sleeps
    pub awake_ticks: u32,
    /// the game's clock (ms) when it last changed hands: grabbed, placed or
    /// dropped (the held tile shrinks and grows from it), and where it was
    /// drawn then (it flies from there to the claw or the square)
    pub since_ms: i64,
    pub from: Affine,
}

impl Tile {
    pub fn new(id: TileId, letter: char, value: i32, pose: Affine, face_down: bool) -> Tile {
        Tile {
            id,
            letter,
            value,
            state: State::Loose { awake: false },
            pose,
            face_down,
            vy: 0,
            slide: V3::ZERO,
            rests_on: Vec::new(),
            awake_ticks: 0,
            since_ms: i64::MIN / 2,
            from: pose,
        }
    }

    pub fn is_loose(&self) -> bool {
        matches!(self.state, State::Loose { .. })
    }

    pub fn is_held(&self) -> bool {
        matches!(self.state, State::Held { .. })
    }

    /// Where the tile model is drawn: its box's frame, turned over inside
    /// the box when it lies face down.
    pub fn draw_pose(&self) -> Affine {
        draw_pose(&self.pose, self.face_down)
    }

    /// The height (Q16 metres) of its highest face over (x, z), if it is
    /// over the tile at all: one of its two big faces, whichever is higher
    /// there. A tile on its edge has no top.
    pub fn top_at(&self, x: i32, z: i32) -> Option<i32> {
        face_heights(&self.pose, x, z).into_iter().flatten().max()
    }

    /// The points of its underside the physics samples: the middle, the
    /// four edges' middles and the four corners.
    pub fn underside(&self) -> [V3; 9] {
        underside_of(&self.pose)
    }

    /// Its middle, half way up.
    pub fn middle(&self) -> V3 {
        self.pose.apply(v3(0, mm1(THICK_MM / 2), 0))
    }
}

/// A box frame's drawing pose (see [`Tile::draw_pose`]).
pub fn draw_pose(pose: &Affine, face_down: bool) -> Affine {
    if face_down {
        pose.then(&Affine::translate(v3(0, mm1(THICK_MM), 0)))
            .then(&Affine::rot_z(fx::HALF))
    } else {
        *pose
    }
}

/// The heights of a box frame's two big faces over (x, z), where it is
/// over them.
fn face_heights(p: &Affine, x: i32, z: i32) -> [Option<i32>; 2] {
    let half = mm1(SIZE_MM / 2) as i64;
    let det = p.c0.x as i64 * p.c2.z as i64 - p.c2.x as i64 * p.c0.z as i64;
    // under about a fifth of flat (a tile stood on its edge)
    if det.abs() < (ONE as i64 * ONE as i64) / 5 {
        return [None, None];
    }
    [0, mm1(THICK_MM)].map(|v| {
        let dx = x as i64 - p.t.x as i64 - fx::mul(v, p.c1.x) as i64;
        let dz = z as i64 - p.t.z as i64 - fx::mul(v, p.c1.z) as i64;
        let u = (dx * p.c2.z as i64 - p.c2.x as i64 * dz) * ONE as i64 / det;
        let w = (p.c0.x as i64 * dz - dx * p.c0.z as i64) * ONE as i64 / det;
        if u.abs() > half || w.abs() > half {
            return None;
        }
        Some(p.t.y + fx::mul(u as i32, p.c0.y) + fx::mul(v, p.c1.y) + fx::mul(w as i32, p.c2.y))
    })
}

/// The nine sample points of a box frame's underside (its y = 0 face).
pub fn underside_of(p: &Affine) -> [V3; 9] {
    let h = mm1(SIZE_MM / 2);
    let mut out = [V3::ZERO; 9];
    let mut k = 0;
    for a in [-h, 0, h] {
        for b in [-h, 0, h] {
            out[k] = p.apply(v3(a, 0, b));
            k += 1;
        }
    }
    out
}

/// The tumbling's numbers, in the core's units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Physics {
    /// Q16 metres a second squared and a second
    pub gravity: i32,
    pub fall_max: i32,
    pub slide: i32,
    /// tangents (Q16) of the steepest rest that holds, and the steepest a
    /// tile tips while settling
    pub repose: i32,
    pub steepest: i32,
    pub settle_ticks: u32,
}

/// Every tile, by id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tiles {
    pub all: Vec<Tile>,
}

impl Tiles {
    pub fn get(&self, id: TileId) -> Option<&Tile> {
        self.all.get(id as usize)
    }

    /// The highest top of a tile not held over (x, z), leaving out `skip`.
    pub fn top_at(&self, x: i32, z: i32, skip: Option<TileId>) -> Option<i32> {
        let near = mm1(SIZE_MM * 3 / 4);
        self.all
            .iter()
            .filter(|t| Some(t.id) != skip && !t.is_held())
            .filter(|t| (t.pose.t.x - x).abs() < near && (t.pose.t.z - z).abs() < near)
            .filter_map(|t| t.top_at(x, z))
            .max()
    }

    /// What holds up a point at (x, z) whose height is `y`: the ground, or
    /// the highest face of another tile not held that is under it (not more
    /// than [`REST_GAP_MM`] over it).
    pub fn support(&self, ground: &Ground, x: i32, z: i32, y: i32, skip: TileId) -> i32 {
        let near = mm1(SIZE_MM * 3 / 4);
        let gap = mm1(REST_GAP_MM);
        let g = ground.height(x, z);
        self.all
            .iter()
            .filter(|t| t.id != skip && !t.is_held())
            .filter(|t| (t.pose.t.x - x).abs() < near && (t.pose.t.z - z).abs() < near)
            .flat_map(|t| face_heights(&t.pose, x, z))
            .flatten()
            .filter(|h| *h <= y + gap)
            .fold(g, i32::max)
    }

    /// The tiles tile `i` lies on now: each whose face is just under a
    /// point of its underside.
    pub fn lying_on(&self, i: usize) -> Vec<TileId> {
        let me = &self.all[i];
        let gap = mm1(REST_GAP_MM);
        let pts = me.underside();
        let near = mm1(SIZE_MM * 3 / 2);
        self.all
            .iter()
            .filter(|t| t.id != me.id && !t.is_held())
            .filter(|t| (t.pose.t.x - me.pose.t.x).abs() < near && (t.pose.t.z - me.pose.t.z).abs() < near)
            .filter(|t| {
                pts.iter().any(|p| {
                    face_heights(&t.pose, p.x, p.z)
                        .into_iter()
                        .flatten()
                        .any(|h| h <= p.y + gap && h >= p.y - gap)
                })
            })
            .map(|t| t.id)
            .collect()
    }

    /// Works out what every loose tile lies on, as the game starts.
    pub fn find_rests(&mut self) {
        for i in 0..self.all.len() {
            if self.all[i].is_loose() {
                self.all[i].rests_on = self.lying_on(i);
            }
        }
    }

    /// Wakes every sleeping loose tile that lay on tile `id`.
    pub fn wake_above(&mut self, id: TileId) {
        for t in &mut self.all {
            if t.state == (State::Loose { awake: false }) && t.rests_on.contains(&id) {
                t.state = State::Loose { awake: true };
                t.awake_ticks = 0;
                t.vy = 0;
            }
        }
    }

    /// Whether any tile is tumbling.
    pub fn any_awake(&self) -> bool {
        self.all.iter().any(|t| t.state == State::Loose { awake: true })
    }

    /// One tick of `dt_ms` for every tumbling tile, in id order. Returns
    /// the tiles that came to rest.
    pub fn step(&mut self, ground: &Ground, ph: &Physics, dt_ms: i32) -> Vec<TileId> {
        let per_tick = |v: i32| (v as i64 * dt_ms as i64 / 1000) as i32;
        let mut settled = Vec::new();
        for i in 0..self.all.len() {
            if self.all[i].state != (State::Loose { awake: true }) {
                continue;
            }
            let id = self.all[i].id;
            let t = &mut self.all[i];
            t.awake_ticks += 1;
            let before = t.vy;
            t.vy = (t.vy - per_tick(ph.gravity)).max(-ph.fall_max);
            let fall = per_tick((before + t.vy) / 2);
            t.pose.t = t.pose.t.add(v3(per_tick(t.slide.x), fall, per_tick(t.slide.z)));
            let pose = t.pose;
            let force = t.awake_ticks >= ph.settle_ticks;
            // touching: a point of the underside at or under what holds it
            let touching = underside_of(&pose)
                .iter()
                .any(|p| self.support(ground, p.x, p.z, p.y, id) >= p.y);
            self.wake_above(id);
            if !touching && !force {
                continue;
            }
            let (pose, slope) = self.settle(ground, ph, i);
            let t = &mut self.all[i];
            t.pose = pose;
            t.vy = 0;
            if slope > ph.repose && !force {
                // too steep to hold: slide downhill, flat
                let n = pose.c1;
                t.slide = v3(n.x, 0, n.z).with_len(ph.slide);
                continue;
            }
            t.slide = V3::ZERO;
            t.state = State::Loose { awake: false };
            settled.push(id);
            let on = self.lying_on(i);
            self.all[i].rests_on = on;
        }
        settled
    }

    /// Where tile `i` rests on what is under it, and the steepness of that
    /// rest (the tangent, Q16): tipped to the plane through the heights
    /// under its four edges, as steep as `steepest` at most, then raised
    /// until no point of its underside is under what holds it.
    fn settle(&self, ground: &Ground, ph: &Physics, i: usize) -> (Affine, i32) {
        let me = &self.all[i];
        let p = me.pose;
        let h = mm1(SIZE_MM / 2);
        let edge = |a: i32, b: i32| {
            let q = p.apply(v3(a, 0, b));
            (q, self.support(ground, q.x, q.z, q.y, me.id))
        };
        let (xp, sxp) = edge(h, 0);
        let (xm, sxm) = edge(-h, 0);
        let (zp, szp) = edge(0, h);
        let (zm, szm) = edge(0, -h);
        // the plane's gradient (gx, gz) from the two differences across
        let d0 = xp.sub(xm);
        let d2 = zp.sub(zm);
        let (a, b, c, d) = (d0.x as i64, d0.z as i64, d2.x as i64, d2.z as i64);
        let (e0, e2) = ((sxp - sxm) as i64, (szp - szm) as i64);
        let det = a * d - b * c;
        let (mut gx, mut gz) = if det == 0 {
            (0, 0)
        } else {
            (
                ((e0 * d - b * e2) * ONE as i64 / det) as i32,
                ((a * e2 - e0 * c) * ONE as i64 / det) as i32,
            )
        };
        let slope = v3(gx, 0, gz).len();
        if slope > ph.steepest {
            gx = (gx as i64 * ph.steepest as i64 / slope as i64) as i32;
            gz = (gz as i64 * ph.steepest as i64 / slope as i64) as i32;
        }
        let n = v3(-gx, ONE, -gz).norm();
        let mut c0 = p.c0.sub(n.scale(p.c0.dot(n))).norm();
        if c0 == V3::ZERO {
            c0 = v3(ONE, 0, 0);
        }
        let mut pose = Affine {
            c0,
            c1: n,
            c2: c0.cross(n).norm(),
            t: p.t,
        };
        // up until nothing pokes through
        let mut lift = i32::MIN;
        for (q, old) in underside_of(&pose).iter().zip(underside_of(&p).iter()) {
            let s = self.support(ground, q.x, q.z, old.y.max(q.y), me.id);
            lift = lift.max(s - q.y);
        }
        pose.t.y += lift;
        (pose, slope.min(ph.steepest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn lying(id: TileId, at_mm: [i32; 3]) -> Tile {
        Tile::new(id, 'A', 1, Affine::translate(crate::model::mm(at_mm)), false)
    }

    pub(crate) fn physics() -> Physics {
        Physics {
            gravity: mm1(2500),
            fall_max: mm1(1200),
            slide: mm1(250),
            repose: ONE / 2,
            steepest: ONE * 13 / 10,
            settle_ticks: 150,
        }
    }

    #[test]
    fn a_flat_tiles_top_is_its_thickness_over_where_it_lies_and_nothing_beside_it() {
        let t = lying(0, [1000, 100, 0]);
        let top = t.top_at(mm1(1050), mm1(-90)).unwrap();
        assert!((top - mm1(145)).abs() <= 2, "{top}");
        assert_eq!(t.top_at(mm1(1120), 0), None, "past its edge");
        assert!(!t.face_down);
    }

    #[test]
    fn a_tipped_tile_is_higher_on_its_raised_side() {
        let mut t = lying(0, [0, 0, 0]);
        // turned 20 degrees about z: +x rises
        t.pose = t.pose.then(&Affine::rot_z(fx::deg(20)));
        // by hand: the top face sits 45 mm up its tipped axis, so at
        // x = ±50 it is about 66 and 29 mm high
        let lo = t.top_at(mm1(-50), 0).unwrap();
        let hi = t.top_at(mm1(50), 0).unwrap();
        assert!(
            (hi - mm1(66)).abs() < mm1(2) && (lo - mm1(29)).abs() < mm1(2),
            "{lo} {hi}"
        );
        // face down, it is drawn turned over inside the same box
        let mut over = lying(1, [0, 0, 0]);
        over.face_down = true;
        let d = over.draw_pose();
        assert!(d.c1.y < 0 && (d.t.y - mm1(THICK_MM)).abs() <= 1);
    }

    fn pile() -> (Tiles, Ground) {
        // A on the sand, B flat on A, C on B
        let mut ts = Tiles {
            all: vec![lying(0, [0, 0, 0]), lying(1, [30, 45, 0]), lying(2, [60, 90, 20])],
        };
        ts.find_rests();
        (ts, Ground::flat(2000))
    }

    #[test]
    fn each_tile_of_a_pile_knows_what_it_lies_on() {
        let (ts, _) = pile();
        assert_eq!(ts.all[0].rests_on, Vec::<TileId>::new());
        assert_eq!(ts.all[1].rests_on, vec![0]);
        assert_eq!(ts.all[2].rests_on, vec![1]);
    }

    #[test]
    fn pulling_the_bottom_tile_tumbles_the_ones_above_onto_the_sand() {
        let (mut ts, g) = pile();
        ts.all[0].state = State::Held { seat: 0, claw: LEFT };
        ts.wake_above(0);
        assert_eq!(ts.all[1].state, State::Loose { awake: true });
        assert_eq!(
            ts.all[2].state,
            State::Loose { awake: false },
            "C wakes when B moves"
        );
        let ph = physics();
        let mut ticks = 0;
        while ts.any_awake() || ticks == 0 {
            ts.step(&g, &ph, 20);
            ticks += 1;
            assert!(ticks < 400, "it settles");
        }
        // B fell 45 mm to the sand; C followed down onto B
        assert!(ts.all[1].pose.t.y.abs() <= mm1(2), "{}", ts.all[1].pose.t.y);
        assert!(
            (ts.all[2].pose.t.y - mm1(45)).abs() <= mm1(3),
            "{}",
            ts.all[2].pose.t.y
        );
        assert_eq!(ts.all[2].rests_on, vec![1]);
    }

    #[test]
    fn a_settled_tile_sleeps_and_never_sinks_into_what_holds_it() {
        // dropped from 100 mm half over a lying tile: it settles tipped,
        // resting on the tile's edge and the sand
        let mut ts = Tiles {
            all: vec![lying(0, [0, 0, 0]), lying(1, [150, 100, 0])],
        };
        ts.all[1].state = State::Loose { awake: true };
        let g = Ground::flat(2000);
        let ph = physics();
        for _ in 0..400 {
            ts.step(&g, &ph, 20);
        }
        assert!(!ts.any_awake());
        let t = &ts.all[1];
        for p in t.underside() {
            let s = ts.support(&g, p.x, p.z, p.y, 1);
            assert!(p.y >= s - mm1(1), "a point {} under its support {}", p.y, s);
        }
        assert!(t.pose.c1.y < ONE, "it rests tipped");
    }

    #[test]
    fn a_tile_on_a_slope_steeper_than_repose_slides_down_it() {
        // a tile dropped on a stack's edge: steep, so it slides off and lies
        // flatter
        let mut ts = Tiles {
            all: vec![
                lying(0, [0, 0, 0]),
                lying(1, [0, 45, 0]),
                lying(2, [0, 90, 0]),
                lying(3, [170, 200, 0]),
            ],
        };
        ts.find_rests();
        ts.all[3].state = State::Loose { awake: true };
        let g = Ground::flat(2000);
        let ph = physics();
        let x0 = ts.all[3].pose.t.x;
        for _ in 0..600 {
            ts.step(&g, &ph, 20);
        }
        assert!(!ts.any_awake());
        assert!(ts.all[3].pose.t.x > x0, "it slid away from the stack");
    }
}
