//! What the page's renderer draws, and every number it needs to read it.
//! Ported from Honey Hole's `gfx/render.rs`, cut down to what Crabble
//! draws (no lake surface, no instanced props yet) and given see-through
//! faces (SECOND-ORDER R1.61).
//!
//! A frame ([`Frame`]) is:
//! - the view values the shaders take as uniforms (the camera, the fog,
//!   the sun, the water's murk and light shafts, the caustic ripples on
//!   the sand), laid out by [`UNIFORMS`];
//! - the solid triangles posed this frame (`tris`), each vertex its
//!   position, flat normal, colour, glow and alpha (Q16; the shim turns
//!   them into floats);
//! - the see-through triangles (`clear`: a ghost tile, a crab's circle in
//!   the overhead view), sorted far to near from the eye, which the page
//!   draws blended after the solid ones and the sky.
//!
//! Every number the page needs to read these arrays (strides, offsets,
//! where each uniform sits in the view values) is in [`numbers_json`], so
//! the page holds none of its own.

use serde::Deserialize;
use serde_json::json;

use super::fx::{self, div, sin, ONE};
use super::geom::{right_of, v3, Mat4, V3};

/// A colour, each channel 0..=ONE.
pub type Rgb = [i32; 3];

/// From 0–255 channels.
pub fn rgb(c: [i32; 3]) -> Rgb {
    c.map(|v| v * ONE / 255)
}

/// Values per vertex: position xyz, normal xyz, colour rgb, glow (0 lit,
/// ONE unlit), alpha (ONE solid).
pub const VERTEX_VALUES: usize = 11;
pub const BYTES_PER_VALUE: usize = 4;
/// Bytes per vertex.
pub const STRIDE: usize = VERTEX_VALUES * BYTES_PER_VALUE;

/// How many vertices a triangle list holds.
pub fn vertex_count(values: &[i32]) -> u32 {
    (values.len() / VERTEX_VALUES) as u32
}

/// How a face is coloured: its colour, how unlit it is (0 lit, ONE unlit)
/// and how solid (ONE solid, 0 invisible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paint {
    pub rgb: Rgb,
    pub glow: i32,
    pub alpha: i32,
}

impl Paint {
    /// A plain lit, solid colour.
    pub const fn lit(rgb: Rgb) -> Paint {
        Paint { rgb, glow: 0, alpha: ONE }
    }

    /// The same paint, `pct` percent as solid as it was.
    pub fn faded(self, pct: i32) -> Paint {
        Paint {
            alpha: (self.alpha as i64 * pct as i64 / 100) as i32,
            ..self
        }
    }
}

/// Pushes one flat triangle onto a list.
pub fn push_tri(out: &mut Vec<i32>, p: [V3; 3], n: V3, paint: Paint) {
    let c = paint.rgb;
    for q in p {
        out.extend_from_slice(&[q.x, q.y, q.z, n.x, n.y, n.z, c[0], c[1], c[2], paint.glow, paint.alpha]);
    }
}

/// Triangles on their way to a frame: the solid ones, and the see-through
/// ones (alpha under ONE) kept apart, to be sorted and drawn blended last.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tris {
    pub solid: Vec<i32>,
    pub clear: Vec<i32>,
}

impl Tris {
    /// Pushes one flat triangle onto the list its alpha calls for. A
    /// triangle with no alpha at all is not drawn.
    pub fn push(&mut self, p: [V3; 3], n: V3, paint: Paint) {
        if paint.alpha <= 0 {
            return;
        }
        let list = if paint.alpha >= ONE {
            &mut self.solid
        } else {
            &mut self.clear
        };
        push_tri(list, p, n, paint);
    }

    /// Pushes a flat convex polygon as a fan of triangles.
    pub fn push_poly(&mut self, p: &[V3], n: V3, paint: Paint) {
        for k in 1..p.len().saturating_sub(1) {
            self.push([p[0], p[k], p[k + 1]], n, paint);
        }
    }

    pub fn append(&mut self, mut o: Tris) {
        self.solid.append(&mut o.solid);
        self.clear.append(&mut o.clear);
    }
}

/// Values per triangle.
const TRI_VALUES: usize = 3 * VERTEX_VALUES;

/// Orders see-through triangles far to near from `eye` (by the squared
/// distance to each one's middle), so each blends over what lies behind
/// it. Triangles as far as each other keep their order.
pub fn sort_far_to_near(tris: &[i32], eye: V3) -> Vec<i32> {
    let mut order: Vec<(i64, usize)> = tris
        .chunks(TRI_VALUES)
        .enumerate()
        .map(|(i, t)| {
            let mid = |k: usize| (t[k] as i64 + t[VERTEX_VALUES + k] as i64 + t[2 * VERTEX_VALUES + k] as i64) / 3;
            let d = [mid(0) - eye.x as i64, mid(1) - eye.y as i64, mid(2) - eye.z as i64];
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2], i)
        })
        .collect();
    order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut out = Vec::with_capacity(tris.len());
    for (_, i) in order {
        out.extend_from_slice(&tris[i * TRI_VALUES..(i + 1) * TRI_VALUES]);
    }
    out
}

/// A camera: where it is, where it looks, its vertical field of view
/// (angle units) and its near and far planes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Camera {
    pub eye: V3,
    pub fwd: V3,
    pub fov: i32,
    pub near: i32,
    pub far: i32,
}

impl Camera {
    /// The view-projection matrix for a screen `aspect` (width / height, Q16).
    pub fn matrix(&self, aspect: i32) -> Mat4 {
        Mat4::perspective(self.fov, aspect, self.near, self.far).mul(&Mat4::look(self.eye, self.fwd, V3::UP))
    }

    /// How far back a camera with this field of view stands for a ball of
    /// `radius` to fill the height of the screen, with `margin` percent to
    /// spare.
    pub fn fit(fov: i32, radius: i32, margin: i32) -> i32 {
        (div(radius, sin(fov / 2)) as i64 * margin as i64 / 100) as i32
    }
}

fn half() -> i32 {
    50
}

/// The light, from `data/look.json`: 0–255 colours, and the sun's
/// direction (toward the sun) in hundredths.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Light {
    /// the fog's colour, which is the background's at eye level too
    pub fog: [i32; 3],
    pub sun_dir: [i32; 3],
    pub sun: [i32; 3],
    /// the light from above and from below, blended by how a face points
    pub sky: [i32; 3],
    pub ground: [i32; 3],
    /// how much of the fog an unlit (glowing) colour takes, in percent
    #[serde(default = "half")]
    pub glow_fog_pct: i32,
}

impl Light {
    pub fn check(&self) -> Result<(), String> {
        for c in [self.fog, self.sun, self.sky, self.ground] {
            if c.iter().any(|v| !(0..=255).contains(v)) {
                return Err(format!("A light colour {c:?} is outside 0 to 255."));
            }
        }
        if self.sun_dir == [0, 0, 0] {
            return Err("The sun has no direction.".into());
        }
        if !(0..=100).contains(&self.glow_fog_pct) {
            return Err("The glow's share of the fog is outside 0 to 100.".into());
        }
        Ok(())
    }
}

/// The water round the camera, as the shaders take it (Q16): the murk
/// that the fog fades to, lighter looking up and darker looking down, soft
/// light shafts fanning from the surface, and the caustic net the ripples
/// make on faces that look up. [`Air::plain`] is none of it: an even fog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Air {
    /// the murk's colour overhead; the fog's colour is eye level's
    pub zenith: Rgb,
    /// the height of the view direction (-ONE..ONE) where the murk starts
    /// to turn from the fog's colour to the zenith's, and where it has
    pub sky_band: [i32; 2],
    /// the murk's colour looking straight down, and the view direction's
    /// depth (-y, -ONE..ONE) where it starts to turn to it and has
    pub deep: Rgb,
    pub deep_band: [i32; 2],
    /// light shafts round `shaft_dir`: two stripe counts per radian round
    /// it and their phases; then their strength, sharpness (a power) and
    /// the cosines from the light where they fade in and are whole
    pub shaft: [i32; 4],
    pub shaft_look: [i32; 4],
    pub shaft_dir: [i32; 3],
    /// the light's colour, for the shafts and the caustics
    pub sheen: Rgb,
    /// three ripples (k along x, k along z, phase, 0) whose meeting lines
    /// make the caustic net; then its scale (one over the ripples' count),
    /// sharpness (a power), strength on faces looking up, and 0
    pub ripples: [[i32; 4]; 3],
    pub caustic: [i32; 4],
    /// how fast the fog thickens from its start (Q16); 0 is an even S-curve
    pub fog_curve: i32,
}

impl Air {
    /// An even fog of the light's colour: no shafts, no caustics.
    pub fn plain(light: &Light) -> Air {
        let fog = rgb(light.fog);
        Air {
            zenith: fog,
            sky_band: [0, ONE],
            deep: fog,
            deep_band: [0, ONE],
            shaft: [0; 4],
            shaft_look: [0, ONE, 0, ONE],
            shaft_dir: [0, ONE, 0],
            sheen: [0; 3],
            ripples: [[0; 4]; 3],
            caustic: [0, ONE, 0, 0],
            fog_curve: 0,
        }
    }
}

/// The uniforms in the view values, in order: name and size. The first 16
/// values are the view-projection matrix (`uVP`).
pub const UNIFORMS: &[(&str, usize)] = &[
    ("uEye", 3),
    ("uFog", 2),
    ("uFogColour", 3),
    ("uFogCurve", 1),
    ("uSunDir", 3),
    ("uSun", 3),
    ("uSky", 3),
    ("uGround", 3),
    ("uHemi", 2),
    ("uGlow", 1),
    ("uZenith", 3),
    ("uSkyBand", 2),
    ("uDeep", 3),
    ("uDeepBand", 2),
    ("uCamRight", 3),
    ("uCamUp", 3),
    ("uCamFwd", 3),
    ("uCamTan", 2),
    ("uShaft", 4),
    ("uShaftLook", 4),
    ("uShaftDir", 3),
    ("uSheen", 3),
    ("uRipple1", 4),
    ("uRipple2", 4),
    ("uRipple3", 4),
    ("uCaustic", 4),
];

/// The matrix's 16 values come first.
const MATRIX_VALUES: usize = 16;

/// How many view values there are.
pub const VIEW_VALUES: usize = {
    let mut n = MATRIX_VALUES;
    let mut i = 0;
    while i < UNIFORMS.len() {
        n += UNIFORMS[i].1;
        i += 1;
    }
    n
};

/// Where a uniform starts in the view values, and its size.
pub fn uniform_at(name: &str) -> (usize, usize) {
    let mut at = MATRIX_VALUES;
    for (n, size) in UNIFORMS {
        if *n == name {
            return (at, *size);
        }
        at += size;
    }
    panic!("no uniform {name}")
}

/// The view values the page uploads each frame, in [`UNIFORMS`] order
/// after the matrix. `fog` is the fog's start and end distances.
pub fn view_values(cam: &Camera, aspect: i32, fog: (i32, i32), light: &Light, air: &Air) -> Vec<i32> {
    let mut v = cam.matrix(aspect).0.to_vec();
    let mut put = |name: &str, values: &[i32]| {
        debug_assert_eq!((v.len(), values.len()), uniform_at(name), "{name}");
        v.extend_from_slice(values);
    };
    put("uEye", &[cam.eye.x, cam.eye.y, cam.eye.z]);
    put("uFog", &[fog.0, fog.1]);
    put("uFogColour", &rgb(light.fog));
    put("uFogCurve", &[air.fog_curve]);
    let [x, y, z] = light.sun_dir.map(|c| c * ONE / 100);
    let d = v3(x, y, z).norm();
    put("uSunDir", &[d.x, d.y, d.z]);
    put("uSun", &rgb(light.sun));
    put("uSky", &rgb(light.sky));
    put("uGround", &rgb(light.ground));
    put("uHemi", &[ONE / 2, ONE / 2]);
    put("uGlow", &[light.glow_fog_pct * ONE / 100]);
    put("uZenith", &air.zenith);
    put("uSkyBand", &air.sky_band);
    put("uDeep", &air.deep);
    put("uDeepBand", &air.deep_band);
    let fwd = cam.fwd.norm();
    let right = right_of(fwd);
    let up = right.cross(fwd);
    let tan_y = div(sin(cam.fov / 2), fx::cos(cam.fov / 2));
    put("uCamRight", &[right.x, right.y, right.z]);
    put("uCamUp", &[up.x, up.y, up.z]);
    put("uCamFwd", &[fwd.x, fwd.y, fwd.z]);
    put("uCamTan", &[fx::mul(tan_y, aspect), tan_y]);
    put("uShaft", &air.shaft);
    put("uShaftLook", &air.shaft_look);
    put("uShaftDir", &air.shaft_dir);
    put("uSheen", &air.sheen);
    put("uRipple1", &air.ripples[0]);
    put("uRipple2", &air.ripples[1]);
    put("uRipple3", &air.ripples[2]);
    put("uCaustic", &air.caustic);
    v
}

/// One frame for the page: see the module's doc.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frame {
    pub view: Vec<i32>,
    pub tris: Vec<i32>,
    /// see-through triangles, far to near from the eye
    pub clear: Vec<i32>,
}

impl Frame {
    /// A frame of these triangles seen through `view`, the see-through ones
    /// sorted for an eye at `eye`.
    pub fn new(view: Vec<i32>, tris: Tris, eye: V3) -> Frame {
        Frame {
            view,
            clear: sort_far_to_near(&tris.clear, eye),
            tris: tris.solid,
        }
    }

    /// The triangles this frame draws.
    pub fn triangles(&self) -> usize {
        (self.tris.len() + self.clear.len()) / TRI_VALUES
    }
}

/// The drawing buffer's size for a canvas of `css` pixels at a device
/// pixel ratio of `dpr_pct` hundredths, at least one pixel each way.
pub fn buffer_size(css_w: i32, css_h: i32, dpr_pct: i32) -> [i32; 2] {
    [(css_w * dpr_pct / 100).max(1), (css_h * dpr_pct / 100).max(1)]
}

/// Width over height as Q16.
pub fn aspect(size: [i32; 2]) -> i32 {
    fx::ratio(size[0], size[1].max(1))
}

/// Every number the page's renderer needs, as JSON.
pub fn numbers_json() -> String {
    let b = BYTES_PER_VALUE as u32;
    let calls = ["", "uniform1fv", "uniform2fv", "uniform3fv", "uniform4fv"];
    let uniforms: Vec<serde_json::Value> = UNIFORMS
        .iter()
        .map(|(name, size)| json!([name, size, uniform_at(name).0, calls[*size]]))
        .collect();
    let (fog_at, _) = uniform_at("uFogColour");
    json!({
        "vertex": {
            "values": VERTEX_VALUES,
            "stride": STRIDE,
            "attributes": [
                ["aPos", 3, 0], ["aNormal", 3, 3 * b], ["aColour", 3, 6 * b],
                ["aGlow", 1, 9 * b], ["aAlpha", 1, 10 * b]
            ],
        },
        // the sky's one triangle, which covers the screen, and its size
        "sky": [-1, -1, 3, -1, -1, 3],
        "sky_size": 2,
        "matrix": [0, MATRIX_VALUES],
        "uniforms": uniforms,
        "fog_colour": [fog_at, fog_at + 3],
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::fx::{deg, int};

    const LIGHT: Light = Light {
        fog: [255, 0, 0],
        sun_dir: [0, 100, 0],
        sun: [255, 255, 255],
        sky: [0, 0, 255],
        ground: [0, 255, 0],
        glow_fog_pct: 50,
    };

    fn uniform_in(n: &serde_json::Value, name: &str) -> (usize, usize) {
        let u = n["uniforms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|u| u[0] == name)
            .unwrap();
        (u[2].as_u64().unwrap() as usize, u[1].as_u64().unwrap() as usize)
    }

    #[test]
    fn the_view_values_sit_where_the_numbers_say() {
        let cam = Camera {
            eye: v3(int(1), int(2), int(3)),
            fwd: v3(0, 0, -ONE),
            fov: deg(40),
            near: ONE / 10,
            far: int(50),
        };
        let v = view_values(&cam, ONE, (int(4), int(9)), &LIGHT, &Air::plain(&LIGHT));
        assert_eq!(v.len(), VIEW_VALUES);
        let n: serde_json::Value = serde_json::from_str(&numbers_json()).unwrap();
        let slice = |name: &str| {
            let (at, len) = uniform_in(&n, name);
            v[at..at + len].to_vec()
        };
        assert_eq!(slice("uEye"), vec![int(1), int(2), int(3)]);
        assert_eq!(slice("uFog"), vec![int(4), int(9)]);
        // the fog is pure red: 255 of 255 is ONE; it follows the matrix (16),
        // the eye (3) and the fog (2)
        assert_eq!(slice("uFogColour"), vec![ONE, 0, 0]);
        assert_eq!(n["fog_colour"], json!([21, 24]), "the clear colour is the fog's");
        // straight up, already one long
        assert_eq!(slice("uSunDir"), vec![0, ONE, 0]);
        assert_eq!(slice("uHemi"), vec![ONE / 2, ONE / 2]);
        // looking along -z: right is +x, up is +y
        assert_eq!(slice("uCamRight"), vec![ONE, 0, 0]);
        assert_eq!(slice("uCamUp"), vec![0, ONE, 0]);
        assert_eq!(slice("uGlow"), vec![ONE / 2]);
        // a plain fog is the same colour every way, with no shafts or caustics
        assert_eq!(slice("uDeep"), vec![ONE, 0, 0]);
        assert_eq!(slice("uZenith"), vec![ONE, 0, 0]);
        assert_eq!(slice("uShaftLook")[0], 0);
        assert_eq!(slice("uCaustic")[2], 0);
        // eleven values a vertex, four bytes each
        assert_eq!(n["vertex"]["stride"], 44);
        assert_eq!(n["vertex"]["attributes"][4], json!(["aAlpha", 1, 40]));
        // the uniforms end where the values do
        let (at, len) = uniform_in(&n, "uCaustic");
        assert_eq!(at + len, VIEW_VALUES);
    }

    #[test]
    fn a_fitted_camera_stands_back_by_the_radius_over_half_the_view() {
        // sin 30° is a half, so a ball of radius 1 needs 2 back, and 10% more
        let d = Camera::fit(deg(60), ONE, 110);
        assert!((d - ONE * 22 / 10).abs() < 64, "{d}");
    }

    #[test]
    fn the_buffer_is_the_css_size_at_the_pixel_ratio_and_never_empty() {
        assert_eq!(buffer_size(800, 600, 150), [1200, 900]);
        assert_eq!(buffer_size(0, 0, 100), [1, 1]);
        assert_eq!(aspect([1600, 900]), fx::ratio(16, 9));
    }

    #[test]
    fn a_light_outside_its_range_or_with_no_sun_is_refused() {
        assert!(LIGHT.check().is_ok());
        let bad = Light {
            sky: [0, 0, 256],
            ..LIGHT
        };
        assert!(bad.check().is_err());
        let dark = Light {
            sun_dir: [0, 0, 0],
            ..LIGHT
        };
        assert_eq!(dark.check(), Err("The sun has no direction.".into()));
        let glare = Light {
            glow_fog_pct: 101,
            ..LIGHT
        };
        assert!(glare.check().is_err());
    }

    #[test]
    fn a_triangle_is_three_vertices_of_eleven_values() {
        let mut out = Vec::new();
        let paint = Paint {
            rgb: [1, 2, 3],
            glow: ONE,
            alpha: ONE / 4,
        };
        push_tri(&mut out, [V3::ZERO; 3], v3(0, ONE, 0), paint);
        assert_eq!(out.len(), 33);
        assert_eq!(&out[..11], &[0, 0, 0, 0, ONE, 0, 1, 2, 3, ONE, ONE / 4]);
    }

    /// One flat triangle facing +z, `z` along the z axis.
    fn tri_at(t: &mut Tris, z: i32, alpha: i32) {
        let p = [v3(-ONE, 0, z), v3(ONE, 0, z), v3(0, ONE, z)];
        t.push(p, v3(0, 0, ONE), Paint { alpha, ..Paint::lit([0; 3]) });
    }

    #[test]
    fn a_see_through_face_goes_to_the_clear_list_and_an_invisible_one_nowhere() {
        let mut t = Tris::default();
        tri_at(&mut t, 0, ONE);
        tri_at(&mut t, 0, ONE / 2);
        tri_at(&mut t, 0, 0);
        assert_eq!((t.solid.len(), t.clear.len()), (33, 33));
        assert_eq!(t.clear[10], ONE / 2);
        // a faded paint keeps its colour and loses solidity by the percent
        assert_eq!(Paint::lit([7; 3]).faded(40).alpha, ONE * 40 / 100);
    }

    #[test]
    fn see_through_faces_are_drawn_far_to_near_whatever_order_they_came_in() {
        let mut t = Tris::default();
        // at z 2, 9 and 5 metres in front of an eye at the origin; the one
        // at 5 m twice, so equal distances keep their order (alphas 3, 4)
        tri_at(&mut t, int(2), 1);
        tri_at(&mut t, int(9), 2);
        tri_at(&mut t, int(5), 3);
        tri_at(&mut t, int(5), 4);
        let f = Frame::new(Vec::new(), t, V3::ZERO);
        let alphas: Vec<i32> = f.clear.chunks(33).map(|tri| tri[10]).collect();
        assert_eq!(alphas, vec![2, 3, 4, 1]);
        // from the far side (an eye at z 10) the order turns round
        let mut t = Tris::default();
        tri_at(&mut t, int(2), 1);
        tri_at(&mut t, int(9), 2);
        let f = Frame::new(Vec::new(), t, v3(0, 0, int(10)));
        let alphas: Vec<i32> = f.clear.chunks(33).map(|tri| tri[10]).collect();
        assert_eq!(alphas, vec![1, 2]);
        assert_eq!(f.triangles(), 2);
    }

    #[test]
    fn a_polygon_is_a_fan_of_triangles() {
        let mut t = Tris::default();
        let p = [V3::ZERO, v3(ONE, 0, 0), v3(ONE, ONE, 0), v3(0, ONE, 0), v3(-ONE, ONE / 2, 0)];
        t.push_poly(&p, v3(0, 0, ONE), Paint::lit([0; 3]));
        assert_eq!(vertex_count(&t.solid), 9);
        // the third triangle is corners 0, 3 and 4
        assert_eq!(&t.solid[66..69], &[0, 0, 0]);
        assert_eq!(&t.solid[77..80], &[0, ONE, 0]);
        assert_eq!(&t.solid[88..91], &[-ONE, ONE / 2, 0]);
    }
}
