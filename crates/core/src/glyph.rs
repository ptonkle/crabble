//! Letters and digits as flat polygons, laid on a model's label (a tile's
//! letter and point value) in a colour the game picks when it draws, so a
//! letter can turn green when its word is real (SECOND-ORDER R1.62).
//!
//! `data/glyphs.json` is the source of truth:
//!
//! ```text
//! {
//!   "_note":   "for people",
//!   "height":  100,          every glyph spans 0 to height, grid units
//!   "gap":     10,           grid units between two glyphs in a text
//!   "lift_um": 300,          how far off the face the text floats, micrometres
//!   "glyphs": {
//!     "I": { "width": 22, "polys": [ [[0, 0], [22, 0], [22, 100], [0, 100]] ] },
//!     ...
//!   }
//! }
//! ```
//!
//! Each glyph is a set of convex polygons on its own grid, x from 0 to its
//! `width` to the right and y from 0 (the baseline) to `height` up; a hole
//! (in A, B, D, O, P, Q, R, 0, 4, 6, 8, 9) is the space the pieces round it
//! leave. Every letter A to Z and digit 0 to 9 must be there (a tile's
//! value can be 10). Pieces may overlap: they are one colour, at one
//! height.
//!
//! [`Font::stamp`] lays a text in a label ([`crate::model::Label`]): the
//! glyphs side by side, `gap` apart, scaled to be as big as the rectangle
//! allows without stretching, in its middle or against its right edge.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::gfx::fx::ONE;
use crate::gfx::geom::V3;
use crate::gfx::render::{Paint, Tris};
use crate::model::{Align, Label};

/// The characters every font must draw.
pub const NEEDED: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileFont {
    #[serde(rename = "_note", default)]
    #[allow(dead_code)]
    note: String,
    height: i32,
    gap: i32,
    lift_um: i32,
    glyphs: BTreeMap<String, FileGlyph>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileGlyph {
    width: i32,
    polys: Vec<Vec<[i32; 2]>>,
}

/// One glyph: its width and its convex pieces, in grid units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph {
    pub width: i32,
    pub polys: Vec<Vec<[i32; 2]>>,
}

/// The checked glyphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    pub height: i32,
    pub gap: i32,
    /// how far off the face text floats, Q16 metres
    pub lift: i32,
    pub glyphs: BTreeMap<char, Glyph>,
}

/// Which side of the line from `a` to `b` the point `c` lies: positive to
/// the left, 0 on it.
fn side(a: [i32; 2], b: [i32; 2], c: [i32; 2]) -> i64 {
    let (ux, uy) = ((b[0] - a[0]) as i64, (b[1] - a[1]) as i64);
    let (vx, vy) = ((c[0] - a[0]) as i64, (c[1] - a[1]) as i64);
    ux * vy - uy * vx
}

/// Whether a polygon is convex with some area: for every side, all the
/// other corners lie on one side of it or on its line (so corners in a
/// straight run are allowed), and the corners do not all lie in a line.
pub fn convex(p: &[[i32; 2]]) -> bool {
    let n = p.len();
    if n < 3 {
        return false;
    }
    let mut turned = false;
    for i in 0..n {
        let (a, b) = (p[i], p[(i + 1) % n]);
        if a == b {
            return false;
        }
        let sides: Vec<i64> = p.iter().map(|c| side(a, b, *c)).collect();
        if sides.iter().any(|s| *s > 0) && sides.iter().any(|s| *s < 0) {
            return false;
        }
        turned |= sides.iter().any(|s| *s != 0);
    }
    turned
}

/// Reads and checks `data/glyphs.json`.
pub fn parse(json: &str) -> Result<Font, String> {
    let f: FileFont = serde_json::from_str(json).map_err(|e| format!("The glyphs could not be read: {e}"))?;
    if !(1..=1000).contains(&f.height) {
        return Err("The glyphs' height is outside 1 to 1000.".into());
    }
    if !(0..=f.height).contains(&f.gap) {
        return Err("The glyphs' gap is outside 0 to their height.".into());
    }
    if !(0..=5000).contains(&f.lift_um) {
        return Err("The glyphs' lift is outside 0 to 5000 um.".into());
    }
    let mut glyphs = BTreeMap::new();
    for (key, g) in &f.glyphs {
        let mut chars = key.chars();
        let c = match (chars.next(), chars.next()) {
            (Some(c), None) if NEEDED.contains(c) => c,
            _ => return Err(format!("The glyph \"{key}\" is not a capital letter or a digit.")),
        };
        if !(1..=4 * f.height).contains(&g.width) {
            return Err(format!("The glyph \"{key}\" has a width outside 1 to four times its height."));
        }
        if g.polys.is_empty() {
            return Err(format!("The glyph \"{key}\" has no pieces."));
        }
        for (k, p) in g.polys.iter().enumerate() {
            if p.iter().any(|q| !(0..=g.width).contains(&q[0]) || !(0..=f.height).contains(&q[1])) {
                return Err(format!("The glyph \"{key}\" has piece {k} outside its box."));
            }
            if !convex(p) {
                return Err(format!("The glyph \"{key}\" has piece {k} that is not convex."));
            }
        }
        glyphs.insert(
            c,
            Glyph {
                width: g.width,
                polys: g.polys.clone(),
            },
        );
    }
    if let Some(c) = NEEDED.chars().find(|c| !glyphs.contains_key(c)) {
        return Err(format!("The glyphs have no \"{c}\"."));
    }
    Ok(Font {
        height: f.height,
        gap: f.gap,
        lift: (f.lift_um as i64 * ONE as i64 / 1_000_000) as i32,
        glyphs,
    })
}

impl Font {
    /// The width of a text in grid units, or the first character the font
    /// cannot draw.
    pub fn width(&self, text: &str) -> Result<i32, char> {
        let mut w = 0;
        for (i, c) in text.chars().enumerate() {
            let g = self.glyphs.get(&c).ok_or(c)?;
            w += g.width + if i > 0 { self.gap } else { 0 };
        }
        Ok(w)
    }

    /// Lays `text` on `label` in `paint`, appending its triangles to `out`
    /// (see the module's doc). An empty text draws nothing; a character the
    /// font cannot draw is refused by name.
    pub fn stamp(&self, text: &str, label: &Label, paint: Paint, out: &mut Tris) -> Result<(), String> {
        let w = self
            .width(text)
            .map_err(|c| format!("There is no glyph for \"{c}\"."))?;
        if w == 0 {
            return Ok(());
        }
        let h = self.height;
        let [lw, lh] = label.size.map(|s| s as i64);
        // grid units to metres: num / den, whichever of width or height is
        // tighter
        let (num, den) = if lw * h as i64 <= lh * w as i64 {
            (lw, w as i64)
        } else {
            (lh, h as i64)
        };
        let to = |u: i64| (u * num / den) as i32;
        let (bw, bh) = (to(w as i64), to(h as i64));
        let left = match label.align {
            Align::Centre => -bw / 2,
            Align::Right => (lw as i32) / 2 - bw,
        };
        let base = label.at.add(label.out.scale(self.lift));
        let point = |x: i64, y: i64| -> V3 {
            base.add(label.right.scale(left + to(x)))
                .add(label.up.scale(to(y) - bh / 2))
        };
        let mut cursor = 0i64;
        for c in text.chars() {
            let g = &self.glyphs[&c];
            for p in &g.polys {
                let corners: Vec<V3> = p
                    .iter()
                    .map(|q| point(cursor + q[0] as i64, q[1] as i64))
                    .collect();
                out.push_poly(&corners, label.out, paint);
            }
            cursor += (g.width + self.gap) as i64;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::geom::v3;
    use crate::gfx::render::VERTEX_VALUES;
    use serde_json::json;

    /// A font of bars: every needed character is one 20 by 100 bar, but
    /// "L" is a bar and a foot.
    fn bars() -> serde_json::Value {
        let mut glyphs = serde_json::Map::new();
        for c in NEEDED.chars() {
            glyphs.insert(
                c.to_string(),
                json!({ "width": 20, "polys": [[[0, 0], [20, 0], [20, 100], [0, 100]]] }),
            );
        }
        glyphs.insert(
            "L".into(),
            json!({ "width": 60, "polys": [[[0, 0], [20, 0], [20, 100], [0, 100]], [[0, 0], [60, 0], [60, 20], [0, 20]]] }),
        );
        json!({ "height": 100, "gap": 10, "lift_um": 1000, "glyphs": glyphs })
    }

    fn font() -> Font {
        parse(&bars().to_string()).unwrap()
    }

    /// A label in the x-y plane facing +z, 200 by 100 mm.
    fn label(align: Align) -> Label {
        Label {
            at: V3::ZERO,
            right: v3(ONE, 0, 0),
            up: v3(0, ONE, 0),
            out: v3(0, 0, ONE),
            size: [ONE / 5, ONE / 10],
            align,
        }
    }

    /// The corners of every triangle stamped, in whole millimetres, and
    /// their normals.
    fn corners(t: &Tris) -> Vec<[i32; 3]> {
        let mm = |v: i32| (v as i64 * 1000 + ONE as i64 / 2).div_euclid(ONE as i64) as i32;
        t.solid
            .chunks(VERTEX_VALUES)
            .map(|v| [mm(v[0]), mm(v[1]), mm(v[2])])
            .collect()
    }

    fn bounds(t: &Tris) -> ([i32; 3], [i32; 3]) {
        let c = corners(t);
        let lo = [0, 1, 2].map(|k| c.iter().map(|p| p[k]).min().unwrap());
        let hi = [0, 1, 2].map(|k| c.iter().map(|p| p[k]).max().unwrap());
        (lo, hi)
    }

    #[test]
    fn a_glyph_fills_its_label_by_the_tighter_side_in_the_middle() {
        let mut t = Tris::default();
        let ink = Paint::lit([ONE, 0, 0]);
        font().stamp("I", &label(Align::Centre), ink, &mut t).unwrap();
        // one bar, two triangles
        assert_eq!(t.solid.len(), 6 * VERTEX_VALUES);
        // 100 units tall in 100 mm is 1 mm a unit, so the bar is 20 mm wide,
        // centred, 1 mm (1000 um) out of the face
        assert_eq!(bounds(&t), ([-10, -50, 1], [10, 50, 1]));
        // in the game's colour, facing out of the label
        assert_eq!(&t.solid[3..9], &[0, 0, ONE, ONE, 0, 0]);
    }

    #[test]
    fn two_glyphs_sit_a_gap_apart_and_a_wide_text_is_fitted_by_width() {
        let mut t = Tris::default();
        let ink = Paint::lit([0; 3]);
        // "10": two bars 20 wide with a gap of 10: 50 units, 50 mm
        font().stamp("10", &label(Align::Centre), ink, &mut t).unwrap();
        assert_eq!(bounds(&t), ([-25, -50, 1], [25, 50, 1]));
        // the second bar starts 30 mm after the first
        assert_eq!(corners(&t)[6], [5, -50, 1]);
        // "LLLL" is 4 × 60 + 3 × 10 = 270 units: wider than tall for the
        // label, so 200 mm over 270 units, and 74 mm tall (100 × 200 / 270)
        let mut t = Tris::default();
        font().stamp("LLLL", &label(Align::Centre), ink, &mut t).unwrap();
        assert_eq!(bounds(&t), ([-100, -37, 1], [100, 37, 1]));
    }

    #[test]
    fn a_right_aligned_text_ends_at_the_labels_right_edge() {
        let mut t = Tris::default();
        font().stamp("I", &label(Align::Right), Paint::lit([0; 3]), &mut t).unwrap();
        assert_eq!(bounds(&t), ([80, -50, 1], [100, 50, 1]));
    }

    #[test]
    fn a_turned_label_carries_its_text_round() {
        // a tile's top: across +x, up -z, facing +y, 45 mm up
        let top = Label {
            at: v3(0, ONE * 45 / 1000, 0),
            right: v3(ONE, 0, 0),
            up: v3(0, 0, -ONE),
            out: v3(0, ONE, 0),
            ..label(Align::Centre)
        };
        let mut t = Tris::default();
        font().stamp("L", &top, Paint::lit([0; 3]), &mut t).unwrap();
        // "L" is 60 by 100 units: 60 by 100 mm; its foot (y 0 to 20 units)
        // lies toward +z, at the bottom of the label as read
        let (lo, hi) = bounds(&t);
        assert_eq!((lo, hi), ([-30, 46, -50], [30, 46, 50]));
        let foot = &corners(&t)[6..12];
        assert!(foot.iter().all(|p| p[2] >= 30), "{foot:?}");
    }

    #[test]
    fn an_unknown_character_is_refused_and_an_empty_text_draws_nothing() {
        let mut t = Tris::default();
        assert_eq!(
            font().stamp("a", &label(Align::Centre), Paint::lit([0; 3]), &mut t),
            Err("There is no glyph for \"a\".".into())
        );
        font().stamp("", &label(Align::Centre), Paint::lit([0; 3]), &mut t).unwrap();
        assert!(t.solid.is_empty());
    }

    #[test]
    fn a_font_missing_a_character_or_with_a_bad_piece_is_refused() {
        let mut v = bars();
        v["glyphs"].as_object_mut().unwrap().remove("Q");
        assert_eq!(parse(&v.to_string()), Err("The glyphs have no \"Q\".".into()));
        let mut v = bars();
        v["glyphs"]["a"] = v["glyphs"]["A"].clone();
        assert_eq!(
            parse(&v.to_string()),
            Err("The glyph \"a\" is not a capital letter or a digit.".into())
        );
        let mut v = bars();
        v["glyphs"]["A"]["polys"][0] = json!([[0, 0], [20, 0], [21, 100], [0, 100]]);
        assert_eq!(parse(&v.to_string()), Err("The glyph \"A\" has piece 0 outside its box.".into()));
        // a dent: (10, 50) pokes in
        let mut v = bars();
        v["glyphs"]["A"]["polys"][0] = json!([[0, 0], [20, 0], [20, 100], [10, 50], [0, 100]]);
        assert_eq!(parse(&v.to_string()), Err("The glyph \"A\" has piece 0 that is not convex.".into()));
        // three corners in a line have no area
        let mut v = bars();
        v["glyphs"]["A"]["polys"][0] = json!([[0, 0], [10, 0], [20, 0]]);
        assert!(parse(&v.to_string()).is_err());
        let mut v = bars();
        v["glyphs"]["A"]["polys"] = json!([]);
        assert_eq!(parse(&v.to_string()), Err("The glyph \"A\" has no pieces.".into()));
    }

    #[test]
    fn convexity_takes_either_winding_and_straight_runs_but_not_a_star() {
        let square = [[0, 0], [10, 0], [10, 10], [0, 10]];
        assert!(convex(&square));
        let mut back = square;
        back.reverse();
        assert!(convex(&back));
        // a corner in the middle of a side runs straight on
        assert!(convex(&[[0, 0], [5, 0], [10, 0], [10, 10], [0, 10]]));
        // a five-pointed star's corners all turn the same way, but it goes
        // round twice
        let star = [[0, 0], [20, 60], [40, 0], [-10, 40], [50, 40]];
        assert!(!convex(&star));
    }
}
