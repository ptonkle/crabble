//! Scenes: stills for the design journal, composed in data with no game
//! code (SECOND-ORDER R1.65). Each is a JSON file under `data/scenes/`,
//! named by its id, that places model instances and a camera:
//!
//! ```text
//! {
//!   "_note": "for people: what the still shows",
//!   "name": "Board with tiles",
//!   "camera": { "target": [0, 0, 0], "yaw": 30, "pitch": 35, "distance": 2400, "fov": 40 },
//!       or    { "eye": [1200, 1500, 2000], "target": [0, 0, 0] }
//!   "fog": [3000, 12000],                      optional: start and end, mm
//!   "things": [
//!     { "model": "sand-patch" },
//!     { "model": "tile", "at": [0, 0, 0], "turn": [0, 15, 0], "letter": "Q" },
//!     { "model": "tile", "at": [210, 0, 0], "letter": "I", "ink": "real", "face": "down" },
//!     { "model": "crab", "at": [0, 0, -600], "colour": "blue", "anim": "walk_side", "ms": 250 },
//!     { "model": "tile", "on": { "thing": 3, "anchor": "claw_left" }, "letter": "E" },
//!     { "model": "tile", "at": [420, 0, 0], "letter": "S", "alpha": 40 }
//!   ]
//! }
//! ```
//!
//! A thing's `at` is millimetres, `turn` degrees (about z, then x, then y,
//! as a part's pose), `scale` percent (default 100). `colour` fills the
//! model's player slot (red, yellow, blue or purple; red when left out).
//! `anim` and `ms` pose it that far into one of its animations. A model
//! with a `letter` label shows `letter` with its point value (`value`, or
//! the letter's standard value) in `ink` (plain, or real for a real word's
//! green). `face: "down"` turns it over where it lies, and `alpha` (percent)
//! draws it see-through, as the ghost tile is. `on` sets it at an anchor of
//! an earlier thing as that one is posed (a tile in a crab's claw); its
//! `at` and `turn` then count from the anchor.
//!
//! The camera stands `distance` mm from `target`, round it by `yaw` from
//! the front (+z) toward +x, and `pitch` above it; or at `eye`, looking at
//! `target`. `fov` is its vertical field of view in degrees (default 40).
//! The Gallery opens a scene and can turn and zoom about its target.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::{facing, pitch_of, v3, yaw_of, Affine, V3};
use crate::gfx::render::Tris;
use crate::glyph::Font;
use crate::letters;
use crate::look::{Look, INKS, PLAYER_COLOURS};
use crate::model::{self, mm, mm1, Anim, Model};

/// The camera's field of view when a scene names none, in degrees.
pub const FOV_DEG: i32 = 40;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileScene {
    #[serde(rename = "_note", default)]
    note: String,
    name: String,
    camera: FileCamera,
    #[serde(default)]
    fog: Option<[i32; 2]>,
    things: Vec<FileThing>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileCamera {
    target: [i32; 3],
    #[serde(default)]
    eye: Option<[i32; 3]>,
    #[serde(default)]
    yaw: Option<i32>,
    #[serde(default)]
    pitch: Option<i32>,
    #[serde(default)]
    distance: Option<i32>,
    #[serde(default)]
    fov: Option<i32>,
}

fn hundred() -> i32 {
    100
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileThing {
    model: String,
    #[serde(default)]
    at: [i32; 3],
    #[serde(default)]
    turn: [i32; 3],
    #[serde(default = "hundred")]
    scale: i32,
    #[serde(default)]
    colour: Option<String>,
    #[serde(default)]
    anim: Option<String>,
    #[serde(default)]
    ms: u32,
    #[serde(default)]
    letter: Option<String>,
    #[serde(default)]
    value: Option<i32>,
    #[serde(default)]
    ink: Option<String>,
    #[serde(default)]
    face: Option<String>,
    #[serde(default = "hundred")]
    alpha: i32,
    #[serde(default)]
    on: Option<FileOn>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileOn {
    thing: usize,
    anchor: String,
}

/// Where a camera looks from: round `target` by `yaw` and `pitch` (angle
/// units), `distance` away (Q16 metres), with a vertical field of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orbit {
    pub target: V3,
    pub yaw: i32,
    pub pitch: i32,
    pub distance: i32,
    pub fov: i32,
}

impl Orbit {
    /// The eye and the way it looks.
    pub fn eye_fwd(&self) -> (V3, V3) {
        let out = facing(self.yaw, self.pitch);
        (self.target.add(out.scale(self.distance)), out.neg())
    }
}

/// How a model is shown: what fills its slot, its letter, and how it lies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dress {
    /// a seat of `PLAYER_COLOURS`
    pub colour: usize,
    /// the letter and its value on a model with a `letter` label
    pub letter: Option<(char, i32)>,
    /// an index of `INKS`
    pub ink: usize,
    pub face_down: bool,
    /// percent solid
    pub alpha: i32,
}

impl Default for Dress {
    fn default() -> Dress {
        Dress {
            colour: 0,
            letter: None,
            ink: 0,
            face_down: false,
            alpha: 100,
        }
    }
}

/// One placed model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thing {
    pub model: String,
    pub at: V3,
    /// angle units about z, x, y
    pub turn: [i32; 3],
    /// Q16
    pub scale: i32,
    pub anim: Option<(Anim, u32)>,
    pub dress: Dress,
    /// an earlier thing and the anchor this one sits at
    pub on: Option<(usize, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub note: String,
    pub camera: Orbit,
    /// the fog's start and end (Q16 metres), or from the camera's distance
    pub fog: Option<(i32, i32)>,
    pub things: Vec<Thing>,
}

fn in_range(p: [i32; 3]) -> bool {
    p.iter().all(|c| c.abs() <= model::MAX_MM)
}

/// Reads one scene file and checks what it can alone (the models it
/// names are checked by [`Scene::check`]). Every refusal starts with `id`.
pub fn parse(id: &str, json: &str) -> Result<Scene, String> {
    let f: FileScene = serde_json::from_str(json).map_err(|e| format!("{id}: {e}"))?;
    read(id, f).map_err(|e| format!("{id}: {e}"))
}

fn read(id: &str, f: FileScene) -> Result<Scene, String> {
    if f.name.trim().is_empty() {
        return Err("the scene has no name.".into());
    }
    let c = &f.camera;
    let fov = c.fov.unwrap_or(FOV_DEG);
    if !(5..=120).contains(&fov) {
        return Err("the camera's field of view is outside 5 to 120 degrees.".into());
    }
    if !in_range(c.target) || c.eye.is_some_and(|e| !in_range(e)) {
        return Err(format!("the camera reaches past {} mm.", model::MAX_MM));
    }
    let target = mm(c.target);
    let camera = match (c.eye, c.yaw, c.pitch, c.distance) {
        (Some(eye), None, None, None) => {
            let back = mm(eye).sub(target);
            if back == V3::ZERO {
                return Err("the camera's eye is at its target.".into());
            }
            Orbit {
                target,
                yaw: yaw_of(back),
                pitch: pitch_of(back),
                distance: back.len(),
                fov: deg(fov),
            }
        }
        (None, Some(yaw), Some(pitch), Some(distance)) => {
            if !(-89..=89).contains(&pitch) {
                return Err("the camera's pitch is outside -89 to 89 degrees.".into());
            }
            if !(1..=model::MAX_MM).contains(&distance) {
                return Err(format!("the camera's distance is outside 1 to {} mm.", model::MAX_MM));
            }
            Orbit {
                target,
                yaw: deg(yaw),
                pitch: deg(pitch),
                distance: mm1(distance),
                fov: deg(fov),
            }
        }
        _ => return Err("the camera needs an eye, or a yaw, pitch and distance.".into()),
    };
    let fog = match f.fog {
        None => None,
        Some([a, b]) if 0 <= a && a < b && b <= 10 * model::MAX_MM => Some((mm1(a), mm1(b))),
        Some(_) => return Err("the fog must start before it ends, within 300000 mm.".into()),
    };
    if f.things.is_empty() {
        return Err("the scene has no things.".into());
    }
    let mut things = Vec::new();
    for (k, t) in f.things.iter().enumerate() {
        let thing = |e: String| format!("thing {k} ({}): {e}", t.model);
        if !in_range(t.at) || t.turn.iter().any(|a| a.abs() > 720) || !(1..=1000).contains(&t.scale) {
            return Err(thing("its place, turn or scale is out of range.".into()));
        }
        let colour = match &t.colour {
            None => 0,
            Some(c) => PLAYER_COLOURS
                .iter()
                .position(|p| p == c)
                .ok_or_else(|| thing(format!("there is no colour \"{c}\".")))?,
        };
        let ink = match &t.ink {
            None => 0,
            Some(i) => INKS
                .iter()
                .position(|p| p == i)
                .ok_or_else(|| thing(format!("there is no ink \"{i}\".")))?,
        };
        let anim = match &t.anim {
            None => None,
            Some(a) => Some((
                Anim::from_key(a).ok_or_else(|| thing(format!("there is no animation \"{a}\".")))?,
                t.ms,
            )),
        };
        let letter = match (&t.letter, t.value) {
            (None, None) => None,
            (None, Some(_)) => return Err(thing("it has a value but no letter.".into())),
            (Some(l), v) => {
                let mut chars = l.chars();
                let c = match (chars.next(), chars.next()) {
                    (Some(c), None) if letters::value(c).is_some() => c,
                    _ => return Err(thing(format!("\"{l}\" is not one capital letter."))),
                };
                let v = v.unwrap_or_else(|| letters::value(c).unwrap_or(0));
                if !(0..=99).contains(&v) {
                    return Err(thing("its value is outside 0 to 99.".into()));
                }
                Some((c, v))
            }
        };
        let face_down = match t.face.as_deref() {
            None | Some("up") => false,
            Some("down") => true,
            Some(other) => return Err(thing(format!("there is no face \"{other}\"."))),
        };
        if !(1..=100).contains(&t.alpha) {
            return Err(thing("its alpha is outside 1 to 100.".into()));
        }
        let on = match &t.on {
            None => None,
            Some(o) if o.thing < k => Some((o.thing, o.anchor.clone())),
            Some(o) => return Err(thing(format!("it sits on thing {}, which is not listed before it.", o.thing))),
        };
        things.push(Thing {
            model: t.model.clone(),
            at: mm(t.at),
            turn: t.turn.map(deg),
            scale: t.scale * ONE / 100,
            anim,
            dress: Dress {
                colour,
                letter,
                ink,
                face_down,
                alpha: t.alpha,
            },
            on,
        });
    }
    Ok(Scene {
        id: id.to_string(),
        name: f.name,
        note: f.note,
        camera,
        fog,
        things,
    })
}

/// Reads every scene from one JSON object of `{id: scene}` (the build
/// bundles `data/scenes/*.json` so), in id order. No scenes is no error.
pub fn parse_all(json: &str) -> Result<Vec<Scene>, String> {
    let all: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(json).map_err(|e| format!("The scenes could not be read: {e}"))?;
    all.into_iter().map(|(id, v)| parse(&id, &v.to_string())).collect()
}

/// The transform that lays a model `turn`ed, `scale`d and maybe turned
/// face down (about z, keeping it on the floor it stood on) at `at`.
pub fn placing(m: &Model, at: V3, turn: [i32; 3], scale: i32, face_down: bool) -> Affine {
    let mut p = Affine::translate(at)
        .then(&Affine::rot_y(turn[1]))
        .then(&Affine::rot_x(turn[0]))
        .then(&Affine::rot_z(turn[2]))
        .then(&Affine::scale(v3(scale, scale, scale)));
    if face_down {
        let lift = mm1(m.min_mm[1] + m.max_mm[1]);
        p = p
            .then(&Affine::translate(v3(0, lift, 0)))
            .then(&Affine::rot_z(fx::HALF));
    }
    p
}

/// Draws one model, posed by `transforms` and placed by `place`, in its
/// dress: the player's colour, the letter and value on its labels, faded
/// by its alpha.
pub fn draw_dressed(
    m: &Model,
    transforms: &[Affine],
    place: &Affine,
    dress: &Dress,
    look: &Look,
    font: &Font,
    out: &mut Tris,
) -> Result<(), String> {
    let paints: Vec<_> = m
        .paints(look.player(dress.colour))
        .into_iter()
        .map(|p| p.faded(dress.alpha))
        .collect();
    m.draw(transforms, place, &paints, out);
    if let Some((letter, value)) = dress.letter {
        let ink = look.ink(dress.ink).faded(dress.alpha);
        if let Some(l) = m.label("letter", transforms, place) {
            font.stamp(&letter.to_string(), &l, ink, out)?;
        }
        if let Some(l) = m.label("value", transforms, place) {
            font.stamp(&value.to_string(), &l, ink, out)?;
        }
    }
    Ok(())
}

impl Scene {
    /// Checks the models the scene names, their animations and anchors,
    /// and the letters it lays on them.
    pub fn check(&self, models: &[Model], font: &Font) -> Result<(), String> {
        for (k, t) in self.things.iter().enumerate() {
            let thing = |e: String| format!("{}: thing {k} ({}): {e}", self.id, t.model);
            let m = models
                .iter()
                .find(|m| m.id == t.model)
                .ok_or_else(|| thing("there is no such model.".into()))?;
            if let Some((a, _)) = t.anim {
                if m.animation(a).is_none() {
                    return Err(thing(format!("the model has no animation \"{}\".", a.key())));
                }
            }
            if let Some((letter, value)) = t.dress.letter {
                if !m.has_label("letter") {
                    return Err(thing("the model has no letter label.".into()));
                }
                font.width(&format!("{letter}{value}"))
                    .map_err(|c| thing(format!("there is no glyph for \"{c}\".")))?;
            }
            if let Some((on, anchor)) = &t.on {
                let parent = &self.things[*on].model;
                let pm = models.iter().find(|m| &m.id == parent);
                if pm.is_some_and(|pm| pm.anchor(anchor).is_none()) {
                    return Err(thing(format!("thing {on} has no anchor \"{anchor}\".")));
                }
            }
        }
        Ok(())
    }

    /// Draws every thing `ms_more` milliseconds past its own time.
    pub fn draw(&self, models: &[Model], font: &Font, look: &Look, ms_more: u32, out: &mut Tris) -> Result<(), String> {
        // each thing's placing and posed transforms, for things set on it
        let mut placed: Vec<(Affine, Vec<Affine>, usize)> = Vec::new();
        for t in &self.things {
            let (mi, m) = models
                .iter()
                .enumerate()
                .find(|(_, m)| m.id == t.model)
                .ok_or_else(|| format!("{}: there is no model \"{}\".", self.id, t.model))?;
            let poses = m.poses(t.anim.map(|a| a.0), t.anim.map_or(0, |a| a.1.saturating_add(ms_more)));
            let transforms = m.transforms(&poses);
            let base = match &t.on {
                None => Affine::IDENTITY,
                Some((k, anchor)) => {
                    let (place, tr, pm) = &placed[*k];
                    match models[*pm].anchor(anchor) {
                        Some(a) => place.then(&models[*pm].anchor_frame(a, tr)),
                        None => *place,
                    }
                }
            };
            let place = base.then(&placing(m, t.at, t.turn, t.scale, t.dress.face_down));
            draw_dressed(m, &transforms, &place, &t.dress, look, font, out)?;
            placed.push((place, transforms, mi));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
