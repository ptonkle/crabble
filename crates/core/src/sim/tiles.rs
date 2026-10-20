//! The tiles: every one has an id, fixed for the game, and one state
//! (PLAN.md, gameplay 5). A loose tile has a pose (where the middle of its
//! underside is, and its turn, as an [`Affine`] in Q16 metres); a held one
//! sits in a crab's claw; a placed one is on a board square for good.
//!
//! A tile is a light rigid box, [`SIZE_MM`] square and [`THICK_MM`] thick.
//! Its top, wherever it lies, is ground a crab can stand on.

use crate::gfx::fx::{self, ONE};
use crate::gfx::geom::{Affine, V3};
use crate::model::mm1;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// on the sand, a mound or a pile; `awake` while it tumbles
    Loose { awake: bool },
    /// in a crab's claw
    Held { seat: usize, claw: Claw },
    /// on the board for good: by whom, and the how-manyth placement of
    /// the game it was (0 for the centre tile)
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
    /// where it lies (loose or placed); a held tile keeps where it was
    pub pose: Affine,
    /// falling speed, Q16 metres a second (down is negative)
    pub vy: i32,
    /// the tiles it lay on when it last came to rest: when one of them
    /// moves, this one wakes
    pub rests_on: Vec<TileId>,
    /// ticks it has been awake, so a tile that cannot settle still sleeps
    pub awake_ticks: u32,
    /// the game's clock (ms) when it last changed hands: grabbed, placed or
    /// dropped (the held tile shrinks and grows from it)
    pub since_ms: i64,
}

impl Tile {
    pub fn is_loose(&self) -> bool {
        matches!(self.state, State::Loose { .. })
    }

    /// Whether it lies face down (its letter toward the sand).
    pub fn face_down(&self) -> bool {
        self.pose.c1.y < 0
    }

    /// The height (Q16 metres) of its highest face over (x, z), if it is
    /// over the tile at all: one of its two big faces, whichever is higher
    /// there. A tile on its edge has no top.
    pub fn top_at(&self, x: i32, z: i32) -> Option<i32> {
        let p = &self.pose;
        let half = mm1(SIZE_MM / 2) as i64;
        let det = p.c0.x as i64 * p.c2.z as i64 - p.c2.x as i64 * p.c0.z as i64;
        // under about a fifth of flat (a tile stood on its edge)
        if det.abs() < (ONE as i64 * ONE as i64) / 5 {
            return None;
        }
        let mut best: Option<i32> = None;
        for v in [0, mm1(THICK_MM)] {
            let dx = x as i64 - p.t.x as i64 - fx::mul(v, p.c1.x) as i64;
            let dz = z as i64 - p.t.z as i64 - fx::mul(v, p.c1.z) as i64;
            let u = (dx * p.c2.z as i64 - p.c2.x as i64 * dz) * ONE as i64 / det;
            let w = (p.c0.x as i64 * dz - dx * p.c0.z as i64) * ONE as i64 / det;
            if u.abs() > half || w.abs() > half {
                continue;
            }
            let y = p.t.y + fx::mul(u as i32, p.c0.y) + fx::mul(v, p.c1.y) + fx::mul(w as i32, p.c2.y);
            best = Some(best.map_or(y, |b| b.max(y)));
        }
        best
    }

    /// Its middle, half way up.
    pub fn middle(&self) -> V3 {
        self.pose.apply(crate::gfx::geom::v3(0, mm1(THICK_MM / 2), 0))
    }
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

    /// The highest top of a tile at rest (loose or placed) over (x, z),
    /// leaving out `skip`.
    pub fn top_at(&self, x: i32, z: i32, skip: Option<TileId>) -> Option<i32> {
        let near = mm1(SIZE_MM * 3 / 4);
        self.all
            .iter()
            .filter(|t| Some(t.id) != skip && !matches!(t.state, State::Held { .. }))
            .filter(|t| (t.pose.t.x - x).abs() < near && (t.pose.t.z - z).abs() < near)
            .filter_map(|t| t.top_at(x, z))
            .max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::geom::v3;

    pub(crate) fn lying(id: TileId, at_mm: [i32; 3]) -> Tile {
        Tile {
            id,
            letter: 'A',
            value: 1,
            state: State::Loose { awake: false },
            pose: Affine::translate(crate::model::mm(at_mm)),
            vy: 0,
            rests_on: Vec::new(),
            awake_ticks: 0,
            since_ms: 0,
        }
    }

    #[test]
    fn a_flat_tiles_top_is_its_thickness_over_where_it_lies_and_nothing_beside_it() {
        let t = lying(0, [1000, 100, 0]);
        let top = t.top_at(mm1(1050), mm1(-90)).unwrap();
        assert!((top - mm1(145)).abs() <= 2, "{top}");
        assert_eq!(t.top_at(mm1(1120), 0), None, "past its edge");
        assert!(!t.face_down());
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
        // turned over, it is face down and its top is its underside
        let mut over = lying(1, [0, 0, 0]);
        over.pose = over.pose.then(&Affine::rot_z(fx::HALF));
        assert!(over.face_down());
        let _ = v3(0, 0, 0);
    }
}
