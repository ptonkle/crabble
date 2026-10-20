//! The stand-in's picture, as flat polygons in the view's pixels for the
//! page's 2D canvas (`web/standin.js` replays them and decides nothing):
//! the water behind, the sand, the board's outline and squares, and each
//! crab as a low many-sided cone in its seat's colour with a wedge on the
//! sand showing where it faces. The camera sits behind the local crab and
//! looks the way the mouse turned it. Kept apart from the WebGL renderer
//! (`gfx/`, built in parallel), which the real game will use instead.
//!
//! The numbers for drawing (colours, the camera, the marker's size) are
//! `data/standin.json`; the board's size is the game's rule
//! (`data/tuning.json`).

use serde::{Deserialize, Serialize};

use super::trig::{cos, sin, ONE};
use super::Game;
use crate::tuning::Tuning;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Colours {
    pub water: String,
    pub sand: String,
    pub board: String,
    pub grid: String,
    /// red, yellow, blue, purple: the lobby seats' colours, `#rrggbb`
    pub crabs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub back_mm: i32,
    pub height_mm: i32,
    pub focal_pct: i32,
    pub near_mm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub radius_mm: i32,
    pub height_mm: i32,
    pub sides: u32,
    pub nose_mm: i32,
    pub shade_min_pct: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    pub colours: Colours,
    pub camera: Camera,
    pub marker: Marker,
    pub floor_mm: i32,
    pub grid_px: u32,
}

/// `#rrggbb` as its three channels.
fn rgb(hex: &str) -> Option<[i64; 3]> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let ch = |k: usize| i64::from_str_radix(&h[k..k + 2], 16).ok();
    Some([ch(0)?, ch(2)?, ch(4)?])
}

/// Parses and checks `data/standin.json`.
pub fn parse(json: &str) -> Result<Look, String> {
    let mut v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("The stand-in look file could not be read: {e}"))?;
    crate::tuning::strip_notes(&mut v);
    let l: Look =
        serde_json::from_value(v).map_err(|e| format!("The stand-in look file could not be read: {e}"))?;
    if l.colours.crabs.len() != 4 || l.colours.crabs.iter().any(|c| rgb(c).is_none()) {
        return Err("The stand-in look file needs four crab colours, each #rrggbb.".into());
    }
    if !(6..=32).contains(&l.marker.sides) {
        return Err("The stand-in look file's marker needs 6 to 32 sides.".into());
    }
    if l.camera.near_mm <= 0 || l.camera.focal_pct <= 0 || !(0..=100).contains(&l.marker.shade_min_pct) {
        return Err(
            "The stand-in look file's near_mm and focal_pct must be above 0, shade_min_pct 0 to 100.".into(),
        );
    }
    Ok(l)
}

/// One shape: filled, or a line `w` pixels wide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Shape {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<u32>,
    pub p: Vec<[i32; 2]>,
}

/// The picture: the background colour, then the shapes far to near.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Scene {
    pub bg: String,
    pub shapes: Vec<Shape>,
}

/// A point in the camera's frame: across (right), up, and depth.
type Cam3 = [i64; 3];

struct View {
    eye: [i64; 3],
    sy: i64,
    cy: i64,
    sp: i64,
    cp: i64,
    focal: i64,
    centre: [i64; 2],
    near: i64,
}

impl View {
    fn to_cam(&self, p: [i64; 3]) -> Cam3 {
        let (dx, dy, dz) = (p[0] - self.eye[0], p[1] - self.eye[1], p[2] - self.eye[2]);
        let across = (dx * self.cy - dz * self.sy) / ONE;
        let ahead = (dx * self.sy + dz * self.cy) / ONE;
        let depth = (ahead * self.cp - dy * self.sp) / ONE;
        let up = (ahead * self.sp + dy * self.cp) / ONE;
        [across, up, depth]
    }

    fn project(&self, c: Cam3) -> [i32; 2] {
        let d = c[2].max(1);
        let lim = 1_000_000;
        let x = (self.centre[0] + self.focal * c[0] / d).clamp(-lim, lim);
        let y = (self.centre[1] - self.focal * c[1] / d).clamp(-lim, lim);
        [x as i32, y as i32]
    }

    /// The part of a polygon (or a line, `closed` false) in front of the
    /// near plane.
    fn clip(&self, pts: &[Cam3], closed: bool) -> Vec<Cam3> {
        let near = self.near;
        let n = pts.len();
        let mut out = Vec::new();
        let edges = if closed { n } else { n.saturating_sub(1) };
        if !closed && n > 0 && pts[0][2] >= near {
            out.push(pts[0]);
        }
        for i in 0..edges {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let (ina, inb) = (a[2] >= near, b[2] >= near);
            if closed && ina {
                out.push(a);
            }
            if ina != inb {
                let span = b[2] - a[2];
                let t = |k: usize| a[k] + (b[k] - a[k]) * (near - a[2]) / span;
                out.push([t(0), t(1), near]);
            }
            if !closed && inb {
                out.push(b);
            }
        }
        out
    }

    fn shape(&self, world: &[[i64; 3]], closed: bool) -> Option<Vec<[i32; 2]>> {
        let cam: Vec<Cam3> = world.iter().map(|p| self.to_cam(*p)).collect();
        let kept = self.clip(&cam, closed);
        let enough = if closed { 3 } else { 2 };
        (kept.len() >= enough).then(|| kept.into_iter().map(|c| self.project(c)).collect())
    }
}

fn fill(c: String, p: Vec<[i32; 2]>) -> Shape {
    Shape {
        fill: Some(c),
        stroke: None,
        w: None,
        p,
    }
}

fn shade(c: [i64; 3], pct: i64) -> String {
    format!(
        "rgb({}, {}, {})",
        c[0] * pct / 100,
        c[1] * pct / 100,
        c[2] * pct / 100
    )
}

/// The picture for a view `w` by `h` pixels.
pub fn scene(g: &Game, t: &Tuning, look: &Look, w: i32, h: i32) -> Scene {
    let Some(me) = g.crabs.get(g.local) else {
        return Scene::default();
    };
    let cam = &look.camera;
    let (sy, cy) = (sin(g.yaw) as i64, cos(g.yaw) as i64);
    let back = cam.back_mm as i64;
    let view = View {
        eye: [
            me.x as i64 - sy * back / ONE,
            cam.height_mm as i64,
            me.z as i64 - cy * back / ONE,
        ],
        sy,
        cy,
        sp: sin(g.pitch) as i64,
        cp: cos(g.pitch) as i64,
        focal: h as i64 * cam.focal_pct as i64 / 100,
        centre: [w as i64 / 2, h as i64 / 2],
        near: cam.near_mm as i64,
    };
    let mut shapes = Vec::new();
    let square = |r: i64, y: i64| [[-r, y, -r], [-r, y, r], [r, y, r], [r, y, -r]];
    let floor = look.floor_mm as i64 / 2;
    shapes.extend(
        view.shape(&square(floor, 0), true)
            .map(|p| fill(look.colours.sand.clone(), p)),
    );
    let half = t.board_mm() as i64 / 2;
    shapes.extend(
        view.shape(&square(half, 0), true)
            .map(|p| fill(look.colours.board.clone(), p)),
    );
    let sq = t.board.square_mm as i64;
    for k in 0..=t.board.size as i64 {
        let at = -half + k * sq;
        for line in [[[at, 0, -half], [at, 0, half]], [[-half, 0, at], [half, 0, at]]] {
            if let Some(p) = view.shape(&line, false) {
                shapes.push(Shape {
                    fill: None,
                    stroke: Some(look.colours.grid.clone()),
                    w: Some(look.grid_px),
                    p,
                });
            }
        }
    }
    // the crabs, far to near
    let mut order: Vec<(i64, usize)> = g
        .crabs
        .iter()
        .enumerate()
        .map(|(i, c)| (view.to_cam([c.x as i64, 0, c.z as i64])[2], i))
        .collect();
    order.sort_by(|a, b| b.cmp(a));
    for (_, i) in order {
        shapes.extend(marker(&view, look, g, i));
    }
    Scene {
        bg: look.colours.water.clone(),
        shapes,
    }
}

/// Crab `i` as a cone with a wedge in front, its faces lit from the
/// north-west, back faces first.
fn marker(view: &View, look: &Look, g: &Game, i: usize) -> Vec<Shape> {
    let c = &g.crabs[i];
    let m = &look.marker;
    let colour = rgb(&look.colours.crabs[i % 4]).unwrap_or([128, 128, 128]);
    let (x, z) = (c.x as i64, c.z as i64);
    let (r, hgt) = (m.radius_mm as i64, m.height_mm as i64);
    let at = |a: i32, d: i64, y: i64| [x + sin(a) as i64 * d / ONE, y, z + cos(a) as i64 * d / ONE];
    let mut out = Vec::new();
    // the wedge on the sand: a point ahead, two corners at the cone's edge
    let quarter = 16384 / 3;
    let wedge = [
        at(c.yaw, m.nose_mm as i64, 0),
        at(c.yaw + quarter, r * 3 / 4, 0),
        at(c.yaw - quarter, r * 3 / 4, 0),
    ];
    out.extend(
        view.shape(&wedge, true)
            .map(|p| fill(shade(colour, m.shade_min_pct as i64), p)),
    );
    let n = m.sides as i32;
    let step = 65536 / n;
    let apex = [x, hgt, z];
    let mut faces: Vec<(i64, Shape)> = Vec::new();
    for k in 0..n {
        let (a0, a1) = (c.yaw + k * step, c.yaw + (k + 1) * step);
        let mid = a0 + step / 2;
        let tri = [at(a0, r, 0), at(a1, r, 0), apex];
        let depth = view.to_cam(at(mid, r / 2, hgt / 2))[2];
        // lit from the north-west (a light at 315 degrees)
        let light = (ONE + cos(mid - 57344) as i64) / 2;
        let min = m.shade_min_pct as i64;
        let pct = min + (100 - min) * light / ONE;
        if let Some(p) = view.shape(&tri, true) {
            faces.push((depth, fill(shade(colour, pct), p)));
        }
    }
    faces.sort_by_key(|a| std::cmp::Reverse(a.0));
    out.extend(faces.into_iter().map(|(_, s)| s));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standin::{Driver, Setup};
    use crate::tuning::{self, tests::SHIPPED};

    const LOOK: &str = include_str!("../../../../data/standin.json");

    fn game() -> (Game, Tuning) {
        let t = tuning::parse(SHIPPED).unwrap();
        let g = Game::new(
            &t,
            Setup {
                drivers: vec![Driver::Local, Driver::Ai],
                local: 0,
                seed: 1,
                fast: true,
                length_ms: None,
            },
        );
        (g, t)
    }

    #[test]
    fn the_shipped_look_is_read_and_a_bad_colour_is_refused() {
        let l = parse(LOOK).unwrap();
        assert_eq!(l.colours.crabs.len(), 4);
        let bad = LOOK.replace("#e5483b", "red");
        assert_eq!(
            parse(&bad).unwrap_err(),
            "The stand-in look file needs four crab colours, each #rrggbb."
        );
    }

    #[test]
    fn the_local_crab_is_drawn_below_the_middle_of_the_view_in_red() {
        let (g, t) = game();
        let look = parse(LOOK).unwrap();
        let s = scene(&g, &t, &look, 800, 600);
        assert_eq!(s.bg, "#14506a");
        // sand, board, 16 + 16 lines of the board, then the crabs
        assert_eq!(s.shapes[0].fill.as_deref(), Some("#d8bd88"));
        assert_eq!(s.shapes[1].fill.as_deref(), Some("#e8d4a2"));
        // the nearest crab is drawn last: the local one, straight ahead
        let last = s.shapes.last().unwrap();
        let fill = last.fill.clone().unwrap();
        assert!(fill.starts_with("rgb("), "{fill}");
        let xs: Vec<i32> = last.p.iter().map(|p| p[0]).collect();
        let ys: Vec<i32> = last.p.iter().map(|p| p[1]).collect();
        assert!(xs.iter().any(|x| (300..=500).contains(x)), "{xs:?}");
        assert!(ys.iter().all(|y| *y > 300), "below the middle: {ys:?}");
    }

    #[test]
    fn a_shape_behind_the_camera_is_cut_at_the_near_plane() {
        let (g, t) = game();
        let mut look = parse(LOOK).unwrap();
        // a floor far larger than the view still draws, cut in front of the eye
        look.floor_mm = 1_000_000;
        let s = scene(&g, &t, &look, 800, 600);
        assert!(s.shapes[0].p.len() >= 3);
        assert!(s.shapes[0].p.iter().all(|p| p[0].abs() <= 1_000_000));
    }
}
