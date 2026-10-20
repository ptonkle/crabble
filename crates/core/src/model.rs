//! Model files: every crab, tile, board, patch of seafloor, prop and the
//! title card is a JSON file under `data/models/`, named by its id
//! (`tile.json`). The file is the source of truth; no Rust code builds a
//! model. The core parses and checks each file and poses it into
//! triangles for the renderer. Ported from Honey Hole's `model.rs`
//! (SECOND-ORDER R1.60), with see-through colours, label rectangles for
//! letters, animations that play once, and Crabble's kinds.
//!
//! The format, in brief:
//!
//! ```text
//! {
//!   "_note":  "for people: what the model is and where its shape comes from",
//!   "name":   "Tile",                     shown in the Gallery
//!   "kind":   "tile",                     crab, tile, board, seafloor, prop, title
//!   "unit":   "mm",                       every length is whole millimetres
//!   "view":   { "yaw": 30, "pitch": 25 }, the Gallery's opening angle, degrees
//!   "colours": {                          flat colours by name, 0-255 each,
//!     "wood": [222, 184, 128],            or a slot the game fills at draw time:
//!     "shell": { "slot": "player", "shade": 80 },  (shade: percent, default 100)
//!     "glass": { "rgb": [200, 240, 255], "alpha": 40 },  see-through: alpha percent
//!     "lamp": { "rgb": [255, 220, 120], "glow": true }   unlit
//!   },
//!   "parts": [                            rigid pieces, parents before children
//!     { "name": "body", "pivot": [0, 0, 0], "vertices": [[x, y, z], ...] },
//!     { "name": "claw_l", "parent": "body", "pivot": [150, 60, 120], "vertices": [...] }
//!   ],
//!   "faces": [                            flat faces over all parts' vertices,
//!     { "colour": "wood", "v": [0, 1, 2] } numbered in part order; 3 or more
//!   ],                                    corners, convex, either winding
//!   "anchors": {
//!     "claw_left": { "part": "claw_l", "at": [x, y, z] },        a point
//!     "letter": { "part": "slab", "at": [0, 45, -6],             a label: a
//!                 "right": [1, 0, 0], "up": [0, 0, -1],          rectangle the
//!                 "size": [150, 150], "align": "centre" }        core stamps text on
//!     "r0c0": { "part": "slab", "at": [..], "right": .., "up": .., "size": ..,
//!               "text": "3W", "ink": "white" }    a label with its own fixed text
//!   },
//!   "animations": {                       keyframed part poses;
//!     "idle": { "ms": 1400, "ease": "linear", "keys": [   ease: smooth or linear
//!       { "at": 0,   "parts": { "claw_l": { "turn": [0, 18, 0] } } },
//!       { "at": 700, "parts": { "claw_l": { "turn": [0, -18, 0] } } } ] }
//!   }
//! }
//! ```
//!
//! Axes follow glTF: +y up, +z forward (a crab's eyes and claws), +x to the
//! model's left. Lengths are millimetres; a model stands on y = 0 with its
//! middle over the origin. A vertex belongs to the part that lists it, and
//! moves with it; a face may join vertices of two parts and then stretches
//! between them. A part's pose turns it about its pivot (degrees: about z,
//! then x, then y), moves it (`move`, mm) and scales it (`scale`, percent;
//! 0 hides it), on top of its parent's pose. A part a key leaves out is at
//! rest in that key. Between keys the pose eases in and out (`"smooth"`,
//! the default) or moves straight (`"linear"`).
//!
//! Most animations loop: after the last key the pose goes back to the
//! first, which is at 0. The ones the game plays once ([`Anim::once`]:
//! jump, grab_left, grab_right, place_left, place_right) stop at their
//! last key, which may sit at `ms`, and hold it. The game may play two at
//! once ([`Model::layered`]): a later layer takes over only the parts its
//! keys name, so the legs can walk while the claws hold.
//!
//! A colour marked `"glow": true` is unlit: drawn at full brightness
//! whatever the sun and shade, and the fog takes it less. A colour with an
//! `"alpha"` under 100 is see-through, drawn blended after every solid face
//! and sorted far to near (`gfx::render::Tris`); the game can also fade a
//! whole model (a ghost tile) when it draws it.
//!
//! An anchor is a point on a part where another model goes (a held tile
//! at `claw_left`). An anchor with `right`, `up` and `size` is a label: a
//! flat rectangle, `size` mm across `right` and up `up`, centred on `at`,
//! facing out along right × up (`right` and `up` are directions, any
//! length, at right angles). The core lays text in it (`glyph::Font`): the
//! text is as tall as the rectangle allows and sits in its middle
//! (`"align": "centre"`, the default) or against its right edge
//! (`"right"`, for a tile's value). A label may carry its own `text`
//! (capital letters and digits) and `ink` (one of the model's colours,
//! which then need be on no face): it always shows that text, as a
//! board's premium squares show "3W". Each kind's needs say which anchors and
//! animations a file must have ([`Kind::needs`], [`Kind::needs_anchors`]),
//! so a crab without a `jump` is refused by name.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::{v3, Affine, V3};
use crate::gfx::render::{self, Paint, Rgb, Tris};

/// The one length unit model files use.
pub const UNIT: &str = "mm";
/// No coordinate may be further than this from the origin (30 m).
pub const MAX_MM: i32 = 30_000;
/// The longest animation, in milliseconds.
pub const MAX_MS: u32 = 60_000;
/// The colour slots a model may name, filled when it is drawn.
pub const SLOTS: [&str; 1] = ["player"];

/// What a model is for. The Gallery lists them in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Crab,
    Tile,
    Board,
    Seafloor,
    Prop,
    Title,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Crab,
        Kind::Tile,
        Kind::Board,
        Kind::Seafloor,
        Kind::Prop,
        Kind::Title,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Kind::Crab => "crab",
            Kind::Tile => "tile",
            Kind::Board => "board",
            Kind::Seafloor => "seafloor",
            Kind::Prop => "prop",
            Kind::Title => "title",
        }
    }

    /// The animations every model of this kind must have: the game plays
    /// them by name.
    pub fn needs(self) -> &'static [Anim] {
        match self {
            Kind::Crab => &[
                Anim::Idle,
                Anim::WalkSide,
                Anim::WalkFwd,
                Anim::Jump,
                Anim::GrabLeft,
                Anim::GrabRight,
                Anim::Hold,
                Anim::PlaceLeft,
                Anim::PlaceRight,
            ],
            _ => &[],
        }
    }

    /// The anchors every model of this kind must have, and whether each
    /// must be a label (with `right`, `up` and `size`).
    pub fn needs_anchors(self) -> &'static [(&'static str, bool)] {
        match self {
            // where a held tile's middle sits in each claw
            Kind::Crab => &[("claw_left", false), ("claw_right", false)],
            // the letter, and the point value in its corner
            Kind::Tile => &[("letter", true), ("value", true)],
            _ => &[],
        }
    }
}

/// The animations the game knows by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Anim {
    /// a crab standing at ease
    Idle,
    /// a crab scuttling sideways (A and D)
    WalkSide,
    /// a crab shuffling forward or back (W and S)
    WalkFwd,
    /// a crab's floaty jump, once
    Jump,
    /// a claw reaching out and closing on a tile, once
    GrabLeft,
    GrabRight,
    /// the claws raised with tiles in them
    Hold,
    /// a claw setting its tile down, once
    PlaceLeft,
    PlaceRight,
    /// a weed in the current
    Sway,
}

impl Anim {
    pub const ALL: [Anim; 10] = [
        Anim::Idle,
        Anim::WalkSide,
        Anim::WalkFwd,
        Anim::Jump,
        Anim::GrabLeft,
        Anim::GrabRight,
        Anim::Hold,
        Anim::PlaceLeft,
        Anim::PlaceRight,
        Anim::Sway,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Anim::Idle => "idle",
            Anim::WalkSide => "walk_side",
            Anim::WalkFwd => "walk_fwd",
            Anim::Jump => "jump",
            Anim::GrabLeft => "grab_left",
            Anim::GrabRight => "grab_right",
            Anim::Hold => "hold",
            Anim::PlaceLeft => "place_left",
            Anim::PlaceRight => "place_right",
            Anim::Sway => "sway",
        }
    }

    pub fn from_key(key: &str) -> Option<Anim> {
        Anim::ALL.into_iter().find(|a| a.key() == key)
    }

    /// Whether the game plays it once and holds its last key, rather than
    /// looping it.
    pub fn once(self) -> bool {
        matches!(
            self,
            Anim::Jump | Anim::GrabLeft | Anim::GrabRight | Anim::PlaceLeft | Anim::PlaceRight
        )
    }
}

// ---- the file as written ----

fn hundred() -> i32 {
    100
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileModel {
    #[serde(rename = "_note", default)]
    #[allow(dead_code)]
    note: String,
    name: String,
    kind: String,
    unit: String,
    #[serde(default)]
    view: FileView,
    colours: BTreeMap<String, FileColour>,
    parts: Vec<FilePart>,
    faces: Vec<FileFace>,
    #[serde(default)]
    anchors: BTreeMap<String, FileAnchor>,
    #[serde(default)]
    animations: BTreeMap<String, FileAnimation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileView {
    yaw: i32,
    pitch: i32,
}

impl Default for FileView {
    fn default() -> FileView {
        FileView { yaw: 30, pitch: 20 }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum FileColour {
    Rgb([i32; 3]),
    Lit(FileLit),
    Slot(FileSlot),
}

/// A fixed colour written as an object, so it can carry `glow` and `alpha`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileLit {
    rgb: [i32; 3],
    #[serde(default)]
    glow: bool,
    #[serde(default = "hundred")]
    alpha: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileSlot {
    slot: String,
    #[serde(default = "hundred")]
    shade: i32,
    #[serde(default)]
    glow: bool,
    #[serde(default = "hundred")]
    alpha: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePart {
    name: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    pivot: [i32; 3],
    vertices: Vec<[i32; 3]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileFace {
    colour: String,
    v: Vec<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileAnchor {
    part: String,
    at: [i32; 3],
    #[serde(default)]
    right: Option<[i32; 3]>,
    #[serde(default)]
    up: Option<[i32; 3]>,
    #[serde(default)]
    size: Option<[i32; 2]>,
    #[serde(default)]
    align: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    ink: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileAnimation {
    ms: u32,
    #[serde(default = "smooth")]
    ease: String,
    keys: Vec<FileKey>,
}

fn smooth() -> String {
    "smooth".into()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileKey {
    at: u32,
    #[serde(default)]
    parts: BTreeMap<String, FilePose>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePose {
    #[serde(default)]
    turn: [i32; 3],
    #[serde(rename = "move", default)]
    shift: [i32; 3],
    #[serde(default = "hundred")]
    scale: i32,
}

// ---- the checked model ----

/// A face's colour: fixed, or a slot filled when drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colour {
    Rgb(Rgb),
    /// a slot of `SLOTS`, at `shade` percent
    Slot(usize, i32),
}

/// A colour as the file gives it: what it is, whether it glows, and how
/// solid it is (Q16, ONE solid).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swatch {
    pub colour: Colour,
    pub glow: bool,
    pub alpha: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub parent: Option<usize>,
    pub pivot: V3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    pub paint: usize,
    pub v: Vec<usize>,
}

/// Where a label's text sits across its rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Centre,
    Right,
}

/// A label's rectangle on its part, at rest: unit directions across and
/// up, and its width and height (Q16 metres).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelRect {
    pub right: V3,
    pub up: V3,
    pub size: [i32; 2],
    pub align: Align,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub name: String,
    pub part: usize,
    pub at: V3,
    pub label: Option<LabelRect>,
    /// fixed text the label always shows, and the colour (an index of the
    /// model's colours) it is stamped in: a board's "3W"
    pub text: Option<(String, usize)>,
}

/// A label placed in the world: its middle, unit directions across, up
/// and out of the face, and its size (Q16 metres).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Label {
    pub at: V3,
    pub right: V3,
    pub up: V3,
    pub out: V3,
    pub size: [i32; 2],
    pub align: Align,
}

/// One part's pose: turned (angle units about z, x, y), moved, scaled (Q16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pose {
    pub turn: [i32; 3],
    pub shift: V3,
    pub scale: i32,
}

impl Pose {
    pub const REST: Pose = Pose {
        turn: [0; 3],
        shift: V3::ZERO,
        scale: ONE,
    };

    fn blend(&self, o: &Pose, t: i32) -> Pose {
        Pose {
            turn: [0, 1, 2].map(|k| fx::lerp(self.turn[k], o.turn[k], t)),
            shift: self.shift.lerp(o.shift, t),
            scale: fx::lerp(self.scale, o.scale, t),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub at: u32,
    /// one pose per part, in part order
    pub poses: Vec<Pose>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
    pub anim: Anim,
    pub ms: u32,
    /// eased in and out between keys (`"smooth"`), or straight (`"linear"`)
    pub smooth: bool,
    pub keys: Vec<Key>,
    /// by part: whether any key names it (a layer takes over only these)
    pub moves: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// the file's name without `.json`
    pub id: String,
    pub name: String,
    pub kind: Kind,
    /// the Gallery's opening angle (angle units)
    pub yaw: i32,
    pub pitch: i32,
    /// by colour name, in name order
    pub swatches: Vec<Swatch>,
    pub parts: Vec<Part>,
    /// every vertex (metres, Q16) and the part it moves with
    pub verts: Vec<(V3, usize)>,
    pub faces: Vec<Face>,
    pub anchors: Vec<Anchor>,
    /// by `Anim` order
    pub animations: Vec<Animation>,
    /// the rest pose's bounds, in millimetres
    pub min_mm: [i32; 3],
    pub max_mm: [i32; 3],
}

/// Millimetres to the core's metres (Q16).
pub fn mm(p: [i32; 3]) -> V3 {
    let c = |a: i32| (a as i64 * ONE as i64 / 1000) as i32;
    v3(c(p[0]), c(p[1]), c(p[2]))
}

/// One length in millimetres to metres (Q16).
pub fn mm1(a: i32) -> i32 {
    mm([a, 0, 0]).x
}

fn in_range(p: [i32; 3]) -> bool {
    p.iter().all(|c| c.abs() <= MAX_MM)
}

/// Reads and checks one model file. `id` is its file name without `.json`;
/// every refusal starts with it.
pub fn parse(id: &str, json: &str) -> Result<Model, String> {
    let file: FileModel = serde_json::from_str(json).map_err(|e| format!("{id}: {e}"))?;
    check(file)
        .map(|m| Model {
            id: id.to_string(),
            ..m
        })
        .map_err(|e| format!("{id}: {e}"))
}

fn swatch(name: &str, c: &FileColour) -> Result<Swatch, String> {
    let (colour, glow, alpha) = match c {
        FileColour::Rgb(c) => (Err(*c), false, 100),
        FileColour::Lit(l) => (Err(l.rgb), l.glow, l.alpha),
        FileColour::Slot(s) => {
            let k = SLOTS
                .iter()
                .position(|k| *k == s.slot)
                .ok_or_else(|| format!("the colour \"{name}\" names no slot \"{}\".", s.slot))?;
            if !(0..=200).contains(&s.shade) {
                return Err(format!("the colour \"{name}\" has a shade outside 0 to 200."));
            }
            (Ok(Colour::Slot(k, s.shade)), s.glow, s.alpha)
        }
    };
    let colour = match colour {
        Ok(c) => c,
        Err(c) if c.iter().any(|v| !(0..=255).contains(v)) => {
            return Err(format!("the colour \"{name}\" is outside 0 to 255."))
        }
        Err(c) => Colour::Rgb(render::rgb(c)),
    };
    if !(1..=100).contains(&alpha) {
        return Err(format!("the colour \"{name}\" has an alpha outside 1 to 100."));
    }
    Ok(Swatch {
        colour,
        glow,
        alpha: alpha * ONE / 100,
    })
}

fn label(name: &str, a: &FileAnchor) -> Result<Option<LabelRect>, String> {
    let (right, up, size) = match (a.right, a.up, a.size) {
        (None, None, None) => {
            if a.align.is_some() {
                return Err(format!("the anchor \"{name}\" has an align but is no label."));
            }
            return Ok(None);
        }
        (Some(r), Some(u), Some(s)) => (r, u, s),
        _ => {
            return Err(format!(
                "the anchor \"{name}\" is a label only with all of right, up and size."
            ))
        }
    };
    let dir = |d: [i32; 3]| v3(d[0] * ONE, d[1] * ONE, d[2] * ONE).norm();
    if right == [0; 3] || up == [0; 3] {
        return Err(format!("the label \"{name}\" has no direction."));
    }
    let dot =
        right[0] as i64 * up[0] as i64 + right[1] as i64 * up[1] as i64 + right[2] as i64 * up[2] as i64;
    if dot != 0 {
        return Err(format!(
            "the label \"{name}\" has right and up not at right angles."
        ));
    }
    if size.iter().any(|s| !(1..=MAX_MM).contains(s)) {
        return Err(format!(
            "the label \"{name}\" has a size outside 1 to {MAX_MM} mm."
        ));
    }
    let align = match a.align.as_deref() {
        None | Some("centre") => Align::Centre,
        Some("right") => Align::Right,
        Some(other) => return Err(format!("the label \"{name}\" has no align \"{other}\".")),
    };
    Ok(Some(LabelRect {
        right: dir(right),
        up: dir(up),
        size: [mm1(size[0]), mm1(size[1])],
        align,
    }))
}

fn check(f: FileModel) -> Result<Model, String> {
    if f.unit != UNIT {
        return Err(format!("the unit is \"{}\"; model files use \"{UNIT}\".", f.unit));
    }
    let kind = Kind::ALL
        .into_iter()
        .find(|k| k.key() == f.kind)
        .ok_or_else(|| format!("there is no kind \"{}\".", f.kind))?;
    if f.name.trim().is_empty() {
        return Err("the model has no name.".into());
    }

    // colours, by name
    let colour_names: Vec<&String> = f.colours.keys().collect();
    let swatches = f
        .colours
        .iter()
        .map(|(name, c)| swatch(name, c))
        .collect::<Result<Vec<_>, _>>()?;

    // parts and their vertices
    if f.parts.is_empty() {
        return Err("the model has no parts.".into());
    }
    let mut parts: Vec<Part> = Vec::new();
    let mut verts = Vec::new();
    for p in &f.parts {
        if p.name.trim().is_empty() {
            return Err("a part has no name.".into());
        }
        if parts.iter().any(|q| q.name == p.name) {
            return Err(format!("there are two parts \"{}\".", p.name));
        }
        let parent = match &p.parent {
            None => None,
            Some(name) => Some(parts.iter().position(|q| &q.name == name).ok_or_else(|| {
                format!(
                    "the part \"{}\" has a parent \"{name}\" that is not listed before it.",
                    p.name
                )
            })?),
        };
        if !in_range(p.pivot) || !p.vertices.iter().all(|v| in_range(*v)) {
            return Err(format!("the part \"{}\" reaches past {MAX_MM} mm.", p.name));
        }
        for v in &p.vertices {
            verts.push((*v, parts.len()));
        }
        parts.push(Part {
            name: p.name.clone(),
            parent,
            pivot: mm(p.pivot),
        });
    }

    // faces
    if f.faces.is_empty() {
        return Err("the model has no faces.".into());
    }
    let mut used = vec![false; verts.len()];
    let mut painted = vec![false; swatches.len()];
    let mut faces = Vec::new();
    for (k, face) in f.faces.iter().enumerate() {
        let paint = colour_names
            .iter()
            .position(|n| **n == face.colour)
            .ok_or_else(|| {
                format!(
                    "face {k} names the colour \"{}\", which is not listed.",
                    face.colour
                )
            })?;
        if face.v.len() < 3 {
            return Err(format!("face {k} has fewer than 3 corners."));
        }
        for (i, &v) in face.v.iter().enumerate() {
            if v >= verts.len() {
                return Err(format!("face {k} names vertex {v}; there are {}.", verts.len()));
            }
            if face.v[..i].contains(&v) {
                return Err(format!("face {k} names vertex {v} twice."));
            }
            used[v] = true;
        }
        painted[paint] = true;
        faces.push(Face {
            paint,
            v: face.v.clone(),
        });
    }
    if let Some(v) = used.iter().position(|u| !u) {
        return Err(format!("vertex {v} is in no face."));
    }

    let part_of = |name: &str| parts.iter().position(|p| p.name == name);

    // anchors
    let mut anchors = Vec::new();
    for (name, a) in &f.anchors {
        let part =
            part_of(&a.part).ok_or_else(|| format!("the anchor \"{name}\" names no part \"{}\".", a.part))?;
        if !in_range(a.at) {
            return Err(format!("the anchor \"{name}\" reaches past {MAX_MM} mm."));
        }
        let rect = label(name, a)?;
        let text = match (&a.text, &a.ink) {
            (None, None) => None,
            (Some(_), _) if rect.is_none() => {
                return Err(format!("the anchor \"{name}\" has a text but is no label."))
            }
            (Some(t), Some(ink)) => {
                if t.is_empty() || !t.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
                    return Err(format!(
                        "the label \"{name}\" has a text that is not capital letters and digits."
                    ));
                }
                let paint = colour_names
                    .iter()
                    .position(|c| *c == ink)
                    .ok_or_else(|| format!("the label \"{name}\" has no colour \"{ink}\" for its ink."))?;
                Some((t.clone(), paint))
            }
            _ => return Err(format!("the label \"{name}\" needs both a text and an ink.")),
        };
        anchors.push(Anchor {
            name: name.clone(),
            part,
            at: mm(a.at),
            label: rect,
            text,
        });
    }
    // a colour may be a label's ink instead of a face's
    for a in &anchors {
        if let Some((_, paint)) = a.text {
            painted[paint] = true;
        }
    }
    if let Some(c) = painted.iter().position(|p| !p) {
        return Err(format!("the colour \"{}\" is on no face.", colour_names[c]));
    }
    for (need, is_label) in kind.needs_anchors() {
        match anchors.iter().find(|a| a.name == *need) {
            None => return Err(format!("a {} model needs the anchor \"{need}\".", kind.key())),
            Some(a) if *is_label && a.label.is_none() => {
                return Err(format!(
                    "a {} model's anchor \"{need}\" must be a label.",
                    kind.key()
                ))
            }
            Some(_) => {}
        }
    }

    // animations
    let mut animations = Vec::new();
    for (name, a) in &f.animations {
        let anim = Anim::from_key(name).ok_or_else(|| format!("there is no animation \"{name}\"."))?;
        if a.ms == 0 || a.ms > MAX_MS {
            return Err(format!(
                "the animation \"{name}\" lasts {} ms; it must be 1 to {MAX_MS}.",
                a.ms
            ));
        }
        if a.keys.first().map(|k| k.at) != Some(0) {
            return Err(format!("the animation \"{name}\" must start with a key at 0."));
        }
        // a looping animation goes back to its first key by `ms`; one played
        // once may end on a key at `ms`
        let end_ok = |at: u32| if anim.once() { at <= a.ms } else { at < a.ms };
        let mut keys: Vec<Key> = Vec::new();
        let mut moves = vec![false; parts.len()];
        for key in &a.keys {
            if keys.last().is_some_and(|k| k.at >= key.at) || !end_ok(key.at) {
                return Err(format!(
                    "the animation \"{name}\" has a key at {} out of order or past its end.",
                    key.at
                ));
            }
            let mut poses = vec![Pose::REST; parts.len()];
            for (part, p) in &key.parts {
                let i = part_of(part)
                    .ok_or_else(|| format!("the animation \"{name}\" names no part \"{part}\"."))?;
                if p.turn.iter().any(|t| t.abs() > 720) || !in_range(p.shift) || !(0..=400).contains(&p.scale)
                {
                    return Err(format!("the animation \"{name}\" poses \"{part}\" out of range."));
                }
                poses[i] = Pose {
                    turn: p.turn.map(deg),
                    shift: mm(p.shift),
                    scale: p.scale * ONE / 100,
                };
                moves[i] = true;
            }
            keys.push(Key { at: key.at, poses });
        }
        let smooth = match a.ease.as_str() {
            "smooth" => true,
            "linear" => false,
            other => return Err(format!("the animation \"{name}\" has no ease \"{other}\".")),
        };
        animations.push(Animation {
            anim,
            ms: a.ms,
            smooth,
            keys,
            moves,
        });
    }
    animations.sort_by_key(|a| a.anim);
    for need in kind.needs() {
        if !animations.iter().any(|a| a.anim == *need) {
            return Err(format!(
                "a {} model needs the animation \"{}\".",
                kind.key(),
                need.key()
            ));
        }
    }

    let mut min_mm = [i32::MAX; 3];
    let mut max_mm = [i32::MIN; 3];
    for (v, _) in &verts {
        for k in 0..3 {
            min_mm[k] = min_mm[k].min(v[k]);
            max_mm[k] = max_mm[k].max(v[k]);
        }
    }

    Ok(Model {
        id: String::new(),
        name: f.name,
        kind,
        yaw: deg(f.view.yaw),
        pitch: deg(f.view.pitch),
        swatches,
        parts,
        verts: verts.into_iter().map(|(v, p)| (mm(v), p)).collect(),
        faces,
        anchors,
        animations,
        min_mm,
        max_mm,
    })
}

/// Reads every model from one JSON object of `{id: model}` (the build
/// bundles `data/models/*.json` so). They come back in Gallery order: by
/// kind, then shortest first, then by id.
pub fn parse_all(json: &str) -> Result<Vec<Model>, String> {
    let all: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(json).map_err(|e| format!("The models could not be read: {e}"))?;
    if all.is_empty() {
        return Err("There are no models.".into());
    }
    let mut models = Vec::new();
    for (id, value) in all {
        models.push(parse(&id, &value.to_string())?);
    }
    models.sort_by_key(|m| (m.kind, m.length_mm(), m.id.clone()));
    Ok(models)
}

/// Eases `t` (0..=ONE) in and out: 3t² − 2t³.
fn ease(t: i32) -> i32 {
    let t2 = fx::mul(t, t);
    3 * t2 - 2 * fx::mul(t2, t)
}

impl Model {
    /// Front to back (along z), in millimetres.
    pub fn length_mm(&self) -> i32 {
        self.max_mm[2] - self.min_mm[2]
    }

    /// Bottom to top, in millimetres.
    pub fn height_mm(&self) -> i32 {
        self.max_mm[1] - self.min_mm[1]
    }

    /// Side to side, in millimetres.
    pub fn width_mm(&self) -> i32 {
        self.max_mm[0] - self.min_mm[0]
    }

    /// The middle of the rest pose's bounds, and the radius of a ball
    /// round them.
    pub fn ball(&self) -> (V3, i32) {
        let lo = mm(self.min_mm);
        let hi = mm(self.max_mm);
        (lo.add(hi).scale(ONE / 2), hi.sub(lo).len() / 2)
    }

    pub fn animation(&self, anim: Anim) -> Option<&Animation> {
        self.animations.iter().find(|a| a.anim == anim)
    }

    pub fn anchor(&self, name: &str) -> Option<&Anchor> {
        self.anchors.iter().find(|a| a.name == name)
    }

    /// Whether any face takes the player's colour.
    pub fn has_slot(&self) -> bool {
        self.swatches.iter().any(|s| matches!(s.colour, Colour::Slot(..)))
    }

    /// Whether the model has a label for text.
    pub fn has_label(&self, name: &str) -> bool {
        self.anchor(name).is_some_and(|a| a.label.is_some())
    }

    /// Each part's pose `ms` into `anim`, or the rest pose. A looping
    /// animation wraps; one played once holds its last key.
    pub fn poses(&self, anim: Option<Anim>, ms: u32) -> Vec<Pose> {
        let Some(a) = anim.and_then(|a| self.animation(a)) else {
            return vec![Pose::REST; self.parts.len()];
        };
        let t = if a.anim.once() { ms.min(a.ms) } else { ms % a.ms };
        let i = a.keys.iter().rposition(|k| k.at <= t).unwrap_or(0);
        let from = &a.keys[i];
        let (to, end) = match a.keys.get(i + 1) {
            Some(k) => (k, k.at),
            None if a.anim.once() => return from.poses.clone(),
            None => (&a.keys[0], a.ms),
        };
        let span = (end - from.at).max(1) as i32;
        let s = fx::ratio((t - from.at) as i32, span);
        let s = if a.smooth { ease(s) } else { s };
        from.poses
            .iter()
            .zip(&to.poses)
            .map(|(p, q)| p.blend(q, s))
            .collect()
    }

    /// Several animations at once, each `ms` in: the first sets every part,
    /// and each later one takes over the parts its keys name. An animation
    /// the model lacks changes nothing.
    pub fn layered(&self, layers: &[(Anim, u32)]) -> Vec<Pose> {
        let mut out = vec![Pose::REST; self.parts.len()];
        for (k, (anim, ms)) in layers.iter().enumerate() {
            let Some(a) = self.animation(*anim) else {
                continue;
            };
            let poses = self.poses(Some(*anim), *ms);
            for (i, p) in poses.into_iter().enumerate() {
                if k == 0 || a.moves[i] {
                    out[i] = p;
                }
            }
        }
        out
    }

    /// Each part's transform from rest to `poses`, parents first.
    pub fn transforms(&self, poses: &[Pose]) -> Vec<Affine> {
        let mut out: Vec<Affine> = Vec::with_capacity(self.parts.len());
        for (part, pose) in self.parts.iter().zip(poses) {
            let local = Affine::translate(part.pivot.add(pose.shift))
                .then(&Affine::rot_y(pose.turn[1]))
                .then(&Affine::rot_x(pose.turn[0]))
                .then(&Affine::rot_z(pose.turn[2]))
                .then(&Affine::scale(v3(pose.scale, pose.scale, pose.scale)))
                .then(&Affine::translate(part.pivot.neg()));
            out.push(match part.parent {
                Some(p) => out[p].then(&local),
                None => local,
            });
        }
        out
    }

    /// Where an anchor is under these transforms, as a frame to place
    /// another model by: the anchor part's turn, at the anchor's point.
    pub fn anchor_frame(&self, anchor: &Anchor, transforms: &[Affine]) -> Affine {
        let t = transforms[anchor.part];
        t.at(t.apply(anchor.at))
    }

    /// A label anchor in the world, under these transforms and `place`.
    pub fn label(&self, name: &str, transforms: &[Affine], place: &Affine) -> Option<Label> {
        let a = self.anchor(name)?;
        let r = a.label?;
        let t = place.then(&transforms[a.part]);
        let right = t.apply_dir(r.right).norm();
        let up = t.apply_dir(r.up).norm();
        // a scaled part scales its label
        let grow = t.apply_dir(r.right).len();
        Some(Label {
            at: t.apply(a.at),
            right,
            up,
            out: right.cross(up).norm(),
            size: r.size.map(|s| fx::mul(s, grow)),
            align: r.align,
        })
    }

    /// Each colour's paint, the player's colour filling the slot.
    pub fn paints(&self, player: Rgb) -> Vec<Paint> {
        self.swatches
            .iter()
            .map(|s| Paint {
                rgb: match s.colour {
                    Colour::Rgb(c) => c,
                    Colour::Slot(_, shade) => player.map(|v| (v * shade / 100).clamp(0, ONE)),
                },
                glow: if s.glow { ONE } else { 0 },
                alpha: s.alpha,
            })
            .collect()
    }

    /// Appends the posed model's triangles, placed by `place`, to `out`;
    /// see-through colours go to its clear list.
    pub fn draw(&self, transforms: &[Affine], place: &Affine, paints: &[Paint], out: &mut Tris) {
        let at: Vec<V3> = self
            .verts
            .iter()
            .map(|(v, part)| place.apply(transforms[*part].apply(*v)))
            .collect();
        for face in &self.faces {
            let p: Vec<V3> = face.v.iter().map(|&i| at[i]).collect();
            out.push_poly(&p, newell(&p), paints[face.paint]);
        }
    }
}

/// A polygon's unit normal by Newell's method (sound for any convex,
/// nearly flat polygon, in its winding's direction), in 64 bits.
pub fn newell(p: &[V3]) -> V3 {
    let mut n = [0i64; 3];
    for (i, a) in p.iter().enumerate() {
        let b = p[(i + 1) % p.len()];
        let (ax, ay, az) = (a.x as i64, a.y as i64, a.z as i64);
        let (bx, by, bz) = (b.x as i64, b.y as i64, b.z as i64);
        n[0] += (ay - by) * (az + bz);
        n[1] += (az - bz) * (ax + bx);
        n[2] += (ax - bx) * (ay + by);
    }
    let big = n.iter().map(|c| c.unsigned_abs()).max().unwrap_or(0);
    let mut shift = 0;
    while big >> shift >= 1 << 30 {
        shift += 1;
    }
    v3(
        (n[0] >> shift) as i32,
        (n[1] >> shift) as i32,
        (n[2] >> shift) as i32,
    )
    .norm()
}

#[cfg(test)]
mod tests;
