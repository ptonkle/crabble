//! The ground a crab stands on: a grid of heights built once, at the start
//! of a game, from the drawn faces of the seafloor, the mounds, the rocks
//! and the board (PLAN.md, gameplay 8). A face that looks up or down more
//! than it looks sideways lays its height on the cells under it; the
//! highest face wins. So a crab stands where the sand is drawn, and a
//! second, hand-written ground cannot drift from the models.
//!
//! Heights are Q16 metres (`gfx::fx`); the grid's cells are [`CELL_MM`]
//! across, centred on the board.

use crate::gfx::fx::{self, ONE};
use crate::gfx::render::VERTEX_VALUES;

/// One cell's side, in millimetres.
pub const CELL_MM: i64 = 25;

/// A face counts as ground when its normal's height is over this (Q16): a
/// slope under about 78 degrees.
const FLAT_ENOUGH: i32 = ONE / 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ground {
    /// cells on a side
    n: usize,
    /// the grid's half-width, mm
    half_mm: i64,
    /// heights, Q16 metres, row by row (z), then x
    h: Vec<i32>,
}

fn to_mm(q: i32) -> i64 {
    q as i64 * 1000 / ONE as i64
}

impl Ground {
    /// Flat sand at height 0, `half_mm` each way from the middle.
    pub fn flat(half_mm: i32) -> Ground {
        let n = (2 * half_mm as i64 / CELL_MM).max(2) as usize;
        Ground {
            n,
            half_mm: half_mm as i64,
            h: vec![0; n * n],
        }
    }

    /// The ground under every face of `tris` (a triangle list, as the
    /// renderer takes it) that is flat enough, `half_mm` each way.
    pub fn from_tris(tris: &[i32], half_mm: i32) -> Ground {
        let mut g = Ground::flat(half_mm);
        let mut set = vec![false; g.h.len()];
        for t in tris.as_chunks::<{ 3 * VERTEX_VALUES }>().0 {
            if t[4].abs() < FLAT_ENOUGH {
                continue;
            }
            g.lay(t, &mut set);
        }
        g
    }

    fn lay(&mut self, t: &[i32], set: &mut [bool]) {
        let p: Vec<(i64, i64, i32)> = (0..3)
            .map(|k| {
                let v = &t[k * VERTEX_VALUES..];
                (to_mm(v[0]), to_mm(v[2]), v[1])
            })
            .collect();
        let (x0, z0, y0) = p[0];
        let (x1, z1, y1) = p[1];
        let (x2, z2, y2) = p[2];
        let d = (x1 - x0) * (z2 - z0) - (x2 - x0) * (z1 - z0);
        if d == 0 {
            return;
        }
        let cell = |v: i64| (v + self.half_mm).div_euclid(CELL_MM);
        let lo_x = cell(x0.min(x1).min(x2)).max(0);
        let hi_x = cell(x0.max(x1).max(x2)).min(self.n as i64 - 1);
        let lo_z = cell(z0.min(z1).min(z2)).max(0);
        let hi_z = cell(z0.max(z1).max(z2)).min(self.n as i64 - 1);
        for j in lo_z..=hi_z {
            let cz = j * CELL_MM + CELL_MM / 2 - self.half_mm;
            for i in lo_x..=hi_x {
                let cx = i * CELL_MM + CELL_MM / 2 - self.half_mm;
                let a = (cx - x0) * (z2 - z0) - (x2 - x0) * (cz - z0);
                let b = (x1 - x0) * (cz - z0) - (cx - x0) * (z1 - z0);
                let inside = if d > 0 {
                    a >= 0 && b >= 0 && a + b <= d
                } else {
                    a <= 0 && b <= 0 && a + b >= d
                };
                if !inside {
                    continue;
                }
                let y = y0 as i64 + (a * (y1 - y0) as i64 + b * (y2 - y0) as i64) / d;
                let k = j as usize * self.n + i as usize;
                if !set[k] || y as i32 > self.h[k] {
                    self.h[k] = y as i32;
                    set[k] = true;
                }
            }
        }
    }

    /// The grid's coordinates of a point: whole cell and the fraction past
    /// its middle (Q16), clamped to the grid.
    fn at(&self, v: i32) -> (usize, i32) {
        let q = (v as i64 * 1000 + self.half_mm * ONE as i64) / CELL_MM - ONE as i64 / 2;
        let max = (self.n as i64 - 1) * ONE as i64;
        let q = q.clamp(0, max - 1);
        ((q >> fx::FRAC_BITS) as usize, (q & (ONE as i64 - 1)) as i32)
    }

    fn cell(&self, i: usize, j: usize) -> i32 {
        self.h[j.min(self.n - 1) * self.n + i.min(self.n - 1)]
    }

    /// The height at (x, z) (Q16 metres), blended between the four cells
    /// round it: what a crab stands on.
    pub fn height(&self, x: i32, z: i32) -> i32 {
        let (i, fx_) = self.at(x);
        let (j, fz) = self.at(z);
        let a = fx::lerp(self.cell(i, j), self.cell(i + 1, j), fx_);
        let b = fx::lerp(self.cell(i, j + 1), self.cell(i + 1, j + 1), fx_);
        fx::lerp(a, b, fz)
    }

    /// The height of the one cell holding (x, z), unblended: the sharp
    /// edge a step is measured against (a board's rim is a step, not a
    /// ramp).
    pub fn cell_height(&self, x: i32, z: i32) -> i32 {
        let c = |v: i32| {
            ((v as i64 * 1000 / ONE as i64 + self.half_mm).div_euclid(CELL_MM)).clamp(0, self.n as i64 - 1)
                as usize
        };
        self.cell(c(x), c(z))
    }

    /// Tests: the ground made a ramp rising toward +x at `deg` degrees,
    /// through height 0 at x = 0.
    #[cfg(test)]
    pub(crate) fn tilt_x(&mut self, deg: i32) {
        let a = fx::deg(deg);
        let tan = fx::div(fx::sin(a), fx::cos(a));
        for j in 0..self.n {
            for i in 0..self.n {
                let cx = i as i64 * CELL_MM + CELL_MM / 2 - self.half_mm;
                self.h[j * self.n + i] = fx::mul(crate::model::mm1(cx as i32), tan);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::geom::v3;
    use crate::gfx::render::{Paint, Tris};
    use crate::model::{mm, mm1};

    /// A flat square `half` mm each way at height `y`, as two triangles.
    fn square(t: &mut Tris, cx: i32, cz: i32, half: i32, y: i32) {
        let p = |x: i32, z: i32| mm([cx + x, y, cz + z]);
        let up = v3(0, ONE, 0);
        let paint = Paint::lit([0; 3]);
        t.push([p(-half, -half), p(half, -half), p(half, half)], up, paint);
        t.push([p(-half, -half), p(half, half), p(-half, half)], up, paint);
    }

    #[test]
    fn the_highest_flat_face_is_the_ground_and_the_rest_is_sand() {
        let mut t = Tris::default();
        square(&mut t, 0, 0, 500, 40);
        square(&mut t, 0, 0, 100, 90);
        let g = Ground::from_tris(&t.solid, 2000);
        let near = |a: i32, b: i32| (a - b).abs() <= ONE / 1000;
        assert!(near(g.height(mm1(300), mm1(-300)), mm1(40)));
        assert!(near(g.height(0, 0), mm1(90)));
        assert!(
            near(g.height(mm1(1500), mm1(1500)), 0),
            "off the faces: sand at 0"
        );
        // a wall (its normal level) lays nothing
        let mut wall = Tris::default();
        let p = |x: i32, y: i32| mm([x, y, 0]);
        wall.push(
            [p(-500, 0), p(500, 0), p(0, 800)],
            v3(0, 0, ONE),
            Paint::lit([0; 3]),
        );
        assert_eq!(Ground::from_tris(&wall.solid, 2000).height(0, mm1(10)), 0);
    }

    #[test]
    fn a_step_is_sharp_in_its_cells_and_blended_between_them() {
        let mut t = Tris::default();
        square(&mut t, 1000, 0, 1000, 50);
        let g = Ground::from_tris(&t.solid, 3000);
        // a cell either side of the edge at x = 0
        assert_eq!(g.cell_height(mm1(-13), 0), 0);
        assert!((g.cell_height(mm1(13), 0) - mm1(50)).abs() <= 1);
        let mid = g.height(0, 0);
        assert!(mid > 0 && mid < mm1(50), "blended across the edge");
    }
}
