//! Word marks on the board (BRIEF.md "Words and ownership", SECOND-ORDER
//! R1.150-R1.179): a flat glowing strip in the owner's colour round each
//! word's whole run of squares, not letter by letter; an alternating
//! colour border, in dashes, round a tile that is in words of several
//! owners; and an aqua ring round the square where a held tile's ghost
//! stands. A scene lists the words (`"marks"`, see `crate::scene`):
//!
//! ```text
//! "marks": {
//!   "on": "tiles",                              or "board"
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
//! two squares. The core works out the shared tiles itself: each square in
//! words of two or more different owners, its colours in seat order.
//!
//! `on` says where the strips lie: `"tiles"` on the tiles' tops, over their
//! edges and the gaps between them, with a skirt down to the board; or
//! `"board"` flat on the board round the tiles' feet. A word that is not
//! real is drawn in dashes with gaps, so a real word shows by its outline
//! as well as its green letters (BRIEF.md: never by colour alone). The
//! sizes come from `data/look.json` (`marks`).

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
    ((cell.1 - SQUARES / 2) * SQUARE_MM, (cell.0 - SQUARES / 2) * SQUARE_MM)
}

/// Where the strips lie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lie {
    /// on the board round the tiles' feet
    Board,
    /// on the tiles' tops
    Tiles,
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
        let (dr, dc) = ((self.to.0 - self.from.0).signum(), (self.to.1 - self.from.1).signum());
        let n = (self.to.0 - self.from.0).max(self.to.1 - self.from.1) + 1;
        (0..n).map(|k| (self.from.0 + k * dr, self.from.1 + k * dc)).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marks {
    pub lie: Lie,
    pub words: Vec<Word>,
    pub ghost: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileMarks {
    #[serde(default)]
    on: Option<String>,
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

/// Checks a scene's marks.
pub fn read(f: &FileMarks) -> Result<Marks, String> {
    let lie = match f.on.as_deref() {
        None | Some("tiles") => Lie::Tiles,
        Some("board") => Lie::Board,
        Some(other) => return Err(format!("the marks lie on \"tiles\" or \"board\", not \"{other}\".")),
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
            return Err(word("it must run along a row or a column, from its first square to its last."));
        }
        let owner = PLAYER_COLOURS
            .iter()
            .position(|p| *p == w.owner)
            .ok_or_else(|| word(&format!("there is no colour \"{}\".", w.owner)))?;
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
    Ok(Marks { lie, words, ghost })
}

/// The squares in words of two or more different owners, in row then
/// column order, each with its owners in seat order.
pub fn shared(words: &[Word]) -> Vec<((i32, i32), Vec<usize>)> {
    let mut cells: Vec<(i32, i32)> = words.iter().flat_map(|w| w.cells()).collect();
    cells.sort();
    cells.dedup();
    cells
        .into_iter()
        .filter_map(|c| {
            let mut owners: Vec<usize> = words
                .iter()
                .filter(|w| w.cells().contains(&c))
                .map(|w| w.owner)
                .collect();
            owners.sort();
            owners.dedup();
            (owners.len() > 1).then_some((c, owners))
        })
        .collect()
}

/// The squares in at least one real word: their letters take the real
/// ink.
pub fn real_cells(words: &[Word]) -> Vec<(i32, i32)> {
    let mut cells: Vec<(i32, i32)> = words.iter().filter(|w| w.real).flat_map(|w| w.cells()).collect();
    cells.sort();
    cells.dedup();
    cells
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
    fn pieces(&self) -> Vec<((i32, i32), (i32, i32), i32, i32, i64)> {
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

/// The band round a run of squares from `a` to `b` (inclusive) for this
/// lie, `lift` mm above its height and wider all round (so marks that
/// overlap never lie in one plane).
pub fn band(look: &MarksLook, lie: Lie, a: (i32, i32), b: (i32, i32), lift: i32) -> Band {
    let l = match lie {
        Lie::Board => &look.board,
        Lie::Tiles => &look.tiles,
    };
    let (x0, z0) = square_mm(a);
    let (x1, z1) = square_mm(b);
    let outer = l.inner_mm + l.width_mm;
    Band {
        x: (x0 + x1) / 2,
        z: (z0 + z1) / 2,
        half: [(x1 - x0).abs() / 2 + outer + lift, (z1 - z0).abs() / 2 + outer + lift],
        radius: look.radius_mm,
        width: l.width_mm,
        y: l.y_mm + lift,
        lip: l.lip_mm,
    }
}

/// Draws every mark: the words' bands (each a little above and wider than the one
/// before, so crossing bands never fight), then the shared tiles' dashed
/// borders over them, then the ghost's ring.
pub fn draw(marks: &Marks, look: &Look, out: &mut Tris) {
    let m = &look.marks;
    let mut lift = 0;
    for w in &marks.words {
        let paint = glow(look.players[PLAYER_COLOURS[w.owner]]);
        let pattern: &[Option<Paint>] = if w.real { &[Some(paint)] } else { &[Some(paint), None] };
        band(m, marks.lie, w.from, w.to, lift).draw(m.dash_mm, pattern, out);
        lift += m.step_mm;
    }
    for (cell, owners) in shared(&marks.words) {
        let pattern: Vec<Option<Paint>> = owners
            .iter()
            .map(|o| Some(glow(look.players[PLAYER_COLOURS[*o]])))
            .collect();
        band(m, marks.lie, cell, cell, lift).draw(m.dash_mm, &pattern, out);
        lift += m.step_mm;
    }
    // the ghost's ring is round its foot, whatever the words lie on
    if let Some(g) = marks.ghost {
        let mut ring = band(m, Lie::Board, g, g, 0);
        ring.y += lift + m.step_mm;
        ring.draw(m.dash_mm, &[Some(glow(m.ghost))], out);
    }
}

#[cfg(test)]
mod tests;
