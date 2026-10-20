//! Word marks on the board (BRIEF.md "Words and ownership", SECOND-ORDER
//! R1.150-R1.179, R7.1): an outline in the owner's colour all the way round
//! each word, lying at the board, not over it: a strip on the board at the
//! tiles' feet, a wall up the tiles' outer sides and a rim on their top
//! edges. A real word's outline is solid, one that is not real is dashed
//! (BRIEF.md: never by colour alone). Where two words of one owner cross,
//! their outlines meet: neither runs through the other. The game draws
//! only the words of this browser's seat (`me`; the person, F8: "it only
//! needs to show your own info"); every word still decides which letters
//! turn green. There is also an aqua ring round the square where a held
//! tile's ghost stands. A scene lists the words (`"marks"`, see
//! `crate::scene`):
//!
//! ```text
//! "marks": {
//!   "me": "yellow",                             optional: only yellow's outlines
//!   "words": [
//!     { "from": [7, 5], "to": [7, 9], "owner": "yellow", "real": true },
//!     { "from": [10, 6], "to": [10, 8], "owner": "red" }
//!   ],
//!   "ghost": [11, 6]                            optional
//! }
//! ```
//!
//! A square is [row, column], both 0 to 14; the centre square (7, 7) is the
//! origin and square (r, c) has its middle at x = (c - 7) × 210 mm,
//! z = (r - 7) × 210 mm (the board's top is y = 40 mm, where tiles stand).
//! A word runs along one row or one column, `from` before `to`, at least
//! two squares. The sizes come from `data/look.json` (`marks`).

use serde::Deserialize;

use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::{v3, V3};
use crate::gfx::render::{self, Paint, Tris};
use crate::look::{Look, MarksLook, PLAYER_COLOURS};
use crate::model::mm1;

/// Squares to a side of the board.
pub const SQUARES: i32 = 15;
/// A board square, mm.
pub const SQUARE_MM: i32 = 210;

/// The middle of square (row, column) in the scene frame, mm: (x, z).
pub fn square_mm(cell: (i32, i32)) -> (i32, i32) {
    (
        (cell.1 - SQUARES / 2) * SQUARE_MM,
        (cell.0 - SQUARES / 2) * SQUARE_MM,
    )
}

/// One word on the board: its first and last squares, its owner's seat,
/// and whether it is a real word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Word {
    pub from: (i32, i32),
    pub to: (i32, i32),
    pub owner: usize,
    pub real: bool,
}

impl Word {
    /// Its squares, first to last.
    pub fn cells(&self) -> Vec<(i32, i32)> {
        let (dr, dc) = (
            (self.to.0 - self.from.0).signum(),
            (self.to.1 - self.from.1).signum(),
        );
        let n = (self.to.0 - self.from.0).max(self.to.1 - self.from.1) + 1;
        (0..n)
            .map(|k| (self.from.0 + k * dr, self.from.1 + k * dc))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marks {
    /// whose outlines are drawn (`None`: every owner's)
    pub me: Option<usize>,
    pub words: Vec<Word>,
    pub ghost: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileMarks {
    #[serde(default)]
    me: Option<String>,
    #[serde(default)]
    words: Vec<FileWord>,
    #[serde(default)]
    ghost: Option<[i32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileWord {
    from: [i32; 2],
    to: [i32; 2],
    owner: String,
    #[serde(default)]
    real: bool,
}

fn on_board(c: [i32; 2]) -> bool {
    c.iter().all(|v| (0..SQUARES).contains(v))
}

fn colour(name: &str) -> Option<usize> {
    PLAYER_COLOURS.iter().position(|p| *p == name)
}

/// Checks a scene's marks.
pub fn read(f: &FileMarks) -> Result<Marks, String> {
    let me = match &f.me {
        None => None,
        Some(name) => Some(colour(name).ok_or_else(|| format!("there is no colour \"{name}\" for \"me\"."))?),
    };
    let mut words = Vec::new();
    for (k, w) in f.words.iter().enumerate() {
        let word = |e: &str| format!("word {k}: {e}");
        if !on_board(w.from) || !on_board(w.to) {
            return Err(word("a square is off the board (rows and columns are 0 to 14)."));
        }
        let along_row = w.from[0] == w.to[0] && w.from[1] < w.to[1];
        let along_col = w.from[1] == w.to[1] && w.from[0] < w.to[0];
        if !along_row && !along_col {
            return Err(word(
                "it must run along a row or a column, from its first square to its last.",
            ));
        }
        let owner = colour(&w.owner).ok_or_else(|| word(&format!("there is no colour \"{}\".", w.owner)))?;
        words.push(Word {
            from: (w.from[0], w.from[1]),
            to: (w.to[0], w.to[1]),
            owner,
            real: w.real,
        });
    }
    let ghost = match f.ghost {
        None => None,
        Some(g) if on_board(g) => Some((g[0], g[1])),
        Some(_) => return Err("the ghost's square is off the board.".into()),
    };
    if words.is_empty() && ghost.is_none() {
        return Err("the marks have no words and no ghost.".into());
    }
    Ok(Marks { me, words, ghost })
}

/// The squares whose every word is real: their letters take the real
/// ink. A letter also in a word that is not real stays plain (the person,
/// CR-Q3: "green whne only all real"; SECOND-ORDER R1.180).
pub fn real_cells(words: &[Word]) -> Vec<(i32, i32)> {
    let fake: Vec<(i32, i32)> = words.iter().filter(|w| !w.real).flat_map(|w| w.cells()).collect();
    let mut cells: Vec<(i32, i32)> = words
        .iter()
        .filter(|w| w.real)
        .flat_map(|w| w.cells())
        .filter(|c| !fake.contains(c))
        .collect();
    cells.sort();
    cells.dedup();
    cells
}

/// A square's four sides, by the step to the square beyond each: toward
/// -z (the row before), +x, +z, -x.
pub const SIDES: [(i32, i32); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];

/// One side of one square on a word's outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    pub cell: (i32, i32),
    /// into [`SIDES`]
    pub side: usize,
    pub owner: usize,
    pub real: bool,
}

fn step(c: (i32, i32), side: usize) -> (i32, i32) {
    (c.0 + SIDES[side].0, c.1 + SIDES[side].1)
}

/// The outline edges of the words owned by `me` (every owner's with
/// `None`): each word's squares' sides, less a side that the word itself
/// or another word of the same owner goes on past, so two crossing outlines meet instead of cutting
/// through each other. A side on two words' outlines is drawn once, solid
/// only when both words are real. In square, then side order.
pub fn edges(words: &[Word], me: Option<usize>) -> Vec<Edge> {
    let mine: Vec<&Word> = words.iter().filter(|w| me.is_none_or(|m| w.owner == m)).collect();
    let mut out: Vec<Edge> = Vec::new();
    for w in &mine {
        for c in w.cells() {
            for side in 0..4 {
                let beyond = step(c, side);
                // the next square along the word is inside it
                // a side inside the word: the word itself goes on past it
                let crossed = mine.iter().any(|o| {
                    let oc = o.cells();
                    o.owner == w.owner && oc.contains(&c) && oc.contains(&beyond)
                });
                if crossed {
                    continue;
                }
                match out
                    .iter_mut()
                    .find(|e| e.cell == c && e.side == side && e.owner == w.owner)
                {
                    Some(e) => e.real &= w.real,
                    None => out.push(Edge {
                        cell: c,
                        side,
                        owner: w.owner,
                        real: w.real,
                    }),
                }
            }
        }
    }
    out.sort();
    out
}

/// The stretches along an edge that are drawn, mm from the square's
/// middle: one from `-reach` to `reach` when solid, else `dashes` dashes
/// of `dash_pct` percent of their share of the square, centred in it.
pub fn stretches(real: bool, reach: i32, dashes: i32, dash_pct: i32) -> Vec<(i32, i32)> {
    if real {
        return vec![(-reach, reach)];
    }
    let n = dashes.max(1);
    let period = SQUARE_MM / n;
    let dash = period * dash_pct / 100;
    (0..n)
        .map(|k| {
            let a = -SQUARE_MM / 2 + k * period + (period - dash) / 2;
            (a, a + dash)
        })
        .collect()
}

/// Draws one edge's outline in `paint`, `thick_pct` percent as wide as
/// the look says: the strip on the board at the tile's foot, the wall up
/// its side and the rim on its top edge.
fn draw_edge(e: &Edge, m: &MarksLook, thick_pct: i32, paint: Paint, out: &mut Tris) {
    let o = &m.outline;
    let (x, z) = square_mm(e.cell);
    let (nz, nx) = SIDES[e.side];
    // along the edge: n turned a quarter
    let (tx, tz) = (-nz, nx);
    let grow = |w: i32| w * thick_pct / 100;
    let foot_out = o.wall_mm + grow(o.foot_out_mm - o.wall_mm);
    let rim_in = o.wall_mm - grow(o.wall_mm - o.rim_in_mm);
    let at = |off: i32, a: i32, y: i32| v3(mm1(x + nx * off + tx * a), mm1(y), mm1(z + nz * off + tz * a));
    let up = v3(0, ONE, 0);
    let side = v3(nx * ONE, 0, nz * ONE);
    // the foot reaches past the square's corners, so outlines close round them
    for (a0, a1) in stretches(e.real, foot_out, o.dashes, o.dash_pct) {
        let y = o.foot_y_mm;
        out.push_poly(
            &[
                at(o.wall_mm, a0, y),
                at(foot_out, a0, y),
                at(foot_out, a1, y),
                at(o.wall_mm, a1, y),
            ],
            up,
            paint,
        );
    }
    // the wall and the rim only along the tile, never over a gap
    for (a0, a1) in stretches(e.real, o.wall_mm, o.dashes, o.dash_pct) {
        let (a0, a1) = (a0.max(-o.wall_mm), a1.min(o.wall_mm));
        let (lo, hi) = (o.foot_y_mm, o.top_y_mm);
        out.push_poly(
            &[
                at(o.wall_mm, a0, lo),
                at(o.wall_mm, a1, lo),
                at(o.wall_mm, a1, hi),
                at(o.wall_mm, a0, hi),
            ],
            side,
            paint,
        );
        out.push_poly(
            &[
                at(rim_in, a0, hi),
                at(o.wall_mm, a0, hi),
                at(o.wall_mm, a1, hi),
                at(rim_in, a1, hi),
            ],
            up,
            paint,
        );
    }
}

/// A rounded rectangle band lying flat at height `y`: its middle, its
/// outer half sizes and corner radius, and its width, all mm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Band {
    pub x: i32,
    pub z: i32,
    pub half: [i32; 2],
    pub radius: i32,
    pub width: i32,
    pub y: i32,
    /// how far a skirt hangs from the outer edge (0: none)
    pub lip: i32,
}

/// Corner arc steps.
const ARC_STEPS: i64 = 6;

/// One piece of a band's way round: the corner centre it starts from, the
/// one it ends at, its angle at its start and end, its middle line's length.
type Piece = ((i32, i32), (i32, i32), i32, i32, i64);

impl Band {
    /// The band's length round its middle line, mm.
    pub fn length(&self) -> i64 {
        let straight = 4 * (self.half[0] - self.radius) as i64 + 4 * (self.half[1] - self.radius) as i64;
        // four quarter arcs of the middle line: 2π(r - w/2)
        straight + (2 * self.radius - self.width) as i64 * 31_416 / 10_000
    }

    /// The pieces round it, anticlockwise from above starting on the +x
    /// side: (corner centre the piece starts from, centre it ends at,
    /// angle at its start and its end, its length on the middle line).
    fn pieces(&self) -> Vec<Piece> {
        let (hx, hz) = (self.half[0] - self.radius, self.half[1] - self.radius);
        let c = [(hx, hz), (-hx, hz), (-hx, -hz), (hx, -hz)];
        let arc = (2 * self.radius - self.width) as i64 * 7_854 / 10_000;
        let mut out = Vec::new();
        for k in 0..4 {
            let prev = c[(k + 3) % 4];
            let len = ((c[k].0 - prev.0).abs() + (c[k].1 - prev.1).abs()) as i64;
            let a = deg(90 * k as i32);
            if len > 0 {
                out.push((prev, c[k], a, a, len));
            }
            out.push((c[k], c[k], a, deg(90 * (k as i32 + 1)), arc));
        }
        out
    }

    /// The outer and inner points (Q16 metres, at y) and the outward
    /// direction `s` mm along the middle line.
    fn at(&self, s: i64) -> (V3, V3, V3) {
        let pieces = self.pieces();
        let mut s0 = 0;
        let mut pick = None;
        for p in &pieces {
            if s < s0 + p.4 {
                pick = Some(*p);
                break;
            }
            s0 += p.4;
        }
        // the very end is the end of the last piece
        let pick = pick.unwrap_or_else(|| {
            let last = pieces[pieces.len() - 1];
            s0 -= last.4;
            last
        });
        let (from, to, a0, a1, len) = pick;
        let t = ((s - s0).clamp(0, len) * ONE as i64 / len.max(1)) as i32;
        let cx = mm1(self.x) + fx::lerp(mm1(from.0), mm1(to.0), t);
        let cz = mm1(self.z) + fx::lerp(mm1(from.1), mm1(to.1), t);
        let a = fx::lerp(a0, a1, t);
        let (dx, dz) = (fx::cos(a), fx::sin(a));
        let y = mm1(self.y);
        let r = mm1(self.radius);
        let ri = mm1(self.radius - self.width);
        (
            v3(cx + fx::mul(r, dx), y, cz + fx::mul(r, dz)),
            v3(cx + fx::mul(ri, dx), y, cz + fx::mul(ri, dz)),
            v3(dx, 0, dz),
        )
    }

    /// Draws the band in dashes of about `dash` mm, taking `pattern`'s
    /// paints in turn (`None` leaves a gap). A pattern of one paint is a
    /// whole band. The number of dashes is a whole number of patterns.
    pub fn draw(&self, dash: i32, pattern: &[Option<Paint>], out: &mut Tris) {
        let total = self.length();
        let n = pattern.len().max(1) as i64;
        let dashes = if n == 1 {
            1
        } else {
            let rounds = (total + dash as i64 * n / 2) / (dash as i64 * n).max(1);
            rounds.max(1) * n
        };
        let mut stations: Vec<i64> = (0..dashes).map(|k| k * total / dashes).collect();
        let mut s0 = 0;
        for p in self.pieces() {
            let steps = if p.2 == p.3 { 1 } else { ARC_STEPS };
            for k in 0..steps {
                stations.push(s0 + p.4 * k / steps);
            }
            s0 += p.4;
        }
        stations.sort();
        stations.dedup();
        stations.push(total);
        let up = v3(0, ONE, 0);
        let floor = mm1(self.y - self.lip);
        for w in stations.windows(2) {
            let mid = (w[0] + w[1]) / 2;
            let k = (mid * dashes / total.max(1)) % n;
            let Some(paint) = pattern[k as usize] else {
                continue;
            };
            let (o0, i0, d0) = self.at(w[0]);
            let (o1, i1, d1) = self.at(w[1]);
            out.push_poly(&[o0, o1, i1, i0], up, paint);
            if self.lip > 0 {
                let down = |p: V3| v3(p.x, floor, p.z);
                let n = d0.add(d1).norm();
                out.push_poly(&[o0, o1, down(o1), down(o0)], n, paint);
            }
        }
    }
}

fn glow(rgb: [i32; 3]) -> Paint {
    Paint {
        rgb: render::rgb(rgb),
        glow: ONE,
        alpha: ONE,
    }
}

/// The ring round square `cell` on the board, `lift` mm above its height
/// and wider all round.
pub fn ring(look: &MarksLook, cell: (i32, i32), lift: i32) -> Band {
    let l = &look.board;
    let (x, z) = square_mm(cell);
    let outer = l.inner_mm + l.width_mm;
    Band {
        x,
        z,
        half: [outer + lift, outer + lift],
        radius: look.radius_mm,
        width: l.width_mm,
        y: l.y_mm + lift,
        lip: l.lip_mm,
    }
}

/// Draws every mark: the outlines of `me`'s words (every owner's when
/// `None`), each in its owner's colour, `thick_pct` percent as wide as the
/// look says, then the ghost's ring.
pub fn draw_thick(marks: &Marks, look: &Look, thick_pct: i32, out: &mut Tris) {
    let m = &look.marks;
    for e in edges(&marks.words, marks.me) {
        let paint = glow(look.players[PLAYER_COLOURS[e.owner]]);
        draw_edge(&e, m, thick_pct, paint, out);
    }
    if let Some(g) = marks.ghost {
        ring(m, g, m.step_mm).draw(m.dash_mm, &[Some(glow(m.ghost))], out);
    }
}

/// [`draw_thick`] at the look's own widths.
pub fn draw(marks: &Marks, look: &Look, out: &mut Tris) {
    draw_thick(marks, look, 100, out)
}

#[cfg(test)]
mod tests;
