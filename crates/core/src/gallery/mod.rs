//! The Gallery: each model on a patch of sand under the water, turning
//! slowly, in any of its animations and player colours (or all four side
//! by side), a tile with any letter, in plain or real-word ink, face up or
//! down; seen from its own angle, the front, the side, behind or above,
//! nearer or further. It also opens the scenes (`crate::scene`), the
//! design journal's stills. Ported from Honey Hole's Gallery
//! (SECOND-ORDER R1.66).
//!
//! It is its own small door, apart from the menus' (`crate::api`), so the
//! menus can change round it: the page holds one [`Gallery`], gives it the
//! files once (`load`), the clock (`advance`) and the canvas's size
//! (`resize`), and sends every control back through `act` with a
//! `gallery.` name. The core decides everything drawn.
//!
//! The page's address can open it straight onto a model or a scene, for
//! the screenshot tool (`scripts/shot.py`): `gallery.query` takes the
//! address's query, e.g. `?model=tile&letter=Q&view=above&ms=0`. Its keys:
//!
//! - `gallery` (open on the first model), `model=<id>` or `scene=<id>`;
//! - `anim=<name>` or `still`; `ms=<n>`: stop the clock there (the
//!   animation, a scene's things and the water all stand at `n` ms, and
//!   the turntable stops), for stills that come out the same every time;
//! - `view=model|front|side|behind|above`, or `yaw=<deg>` and
//!   `pitch=<deg>` to stand anywhere (yaw round from the front toward the
//!   model's left), and `zoom=<percent>` (100 fits the model or keeps the
//!   scene's distance; 200 is twice as near);
//! - `colour=red|yellow|blue|purple|all`, `letter=<A-Z>`, `value=<n>`,
//!   `ink=plain|real`, `face=up|down`, `alpha=<percent>`;
//! - `turn=on|off`, and `bare`: no controls on screen, the 3D view only.
//!
//! Keys it does not know are left for the rest of the page.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::gfx::fx::{self, deg, ONE, TURN};
use crate::gfx::geom::{v3, Affine, V3};
use crate::gfx::render::{self, Camera, Frame, Paint, Tris};
use crate::glyph::{self, Font};
use crate::letters;
use crate::look::{Look, INKS, PLAYER_COLOURS};
use crate::model::{self, mm1, Anim, Model};
use crate::scene::{self, Dress, Orbit, Scene};

/// The words the Gallery needs from `data/gallery.json`.
pub const KEYS: &[&str] = &[
    "heading",
    "none",
    "model",
    "scene",
    "scene.none",
    "move",
    "still",
    "colour",
    "colour.all",
    "letter",
    "ink",
    "face",
    "face.up",
    "face.down",
    "view",
    "view.model",
    "view.front",
    "view.side",
    "view.behind",
    "view.above",
    "turn",
    "zoom.in",
    "zoom.out",
    "back",
    "size",
];

/// The floor's sides (a flat many-sided disc).
const FLOOR_SIDES: i32 = 48;
/// The floor lies this far under a model's lowest point (mm), so the two
/// never fight for the same pixels.
const FLOOR_DROP_MM: i32 = 2;
/// The copies in a row of four stand this far apart, in percent of one
/// copy's width.
const ROW_GAP: i32 = 112;
/// From the front, the side and behind, the camera looks down this much.
const LOW_PITCH: i32 = deg(8);
/// From above it looks this far down (straight down leaves it no "up").
const ABOVE: i32 = deg(89);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileText {
    #[serde(rename = "_note", default)]
    #[allow(dead_code)]
    note: String,
    strings: BTreeMap<String, String>,
}

/// Where the camera stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// the model's own angle (its file's `view`)
    Model,
    Front,
    Side,
    Behind,
    Above,
}

impl View {
    pub const ALL: [View; 5] = [View::Model, View::Front, View::Side, View::Behind, View::Above];

    pub fn key(self) -> &'static str {
        match self {
            View::Model => "model",
            View::Front => "front",
            View::Side => "side",
            View::Behind => "behind",
            View::Above => "above",
        }
    }

    /// The camera's yaw and pitch for a model with this opening angle.
    fn angles(self, m: &Model) -> (i32, i32) {
        match self {
            View::Model => (m.yaw, m.pitch),
            View::Front => (0, LOW_PITCH),
            View::Side => (fx::QUARTER, LOW_PITCH),
            View::Behind => (fx::HALF, LOW_PITCH),
            View::Above => (0, ABOVE),
        }
    }
}

/// What the Gallery shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Show {
    Model(usize),
    Scene(usize),
}

/// The Gallery's choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub show: Show,
    pub anim: Option<Anim>,
    /// a seat of `PLAYER_COLOURS`, or `None` for all four side by side
    pub colour: Option<usize>,
    pub letter: char,
    /// the value drawn with the letter, when not the letter's own
    pub value: Option<i32>,
    /// an index of `INKS`
    pub ink: usize,
    pub face_down: bool,
    /// percent solid
    pub alpha: i32,
    pub view: View,
    /// angles that replace the view's (angle units)
    pub yaw: Option<i32>,
    pub pitch: Option<i32>,
    /// percent: 100 fits, more is nearer
    pub zoom: i32,
    pub turning: bool,
    /// the clock stopped at so many ms (`ms=` in the address)
    pub frozen: Option<u32>,
    pub bare: bool,
}

impl Default for Choice {
    fn default() -> Choice {
        Choice {
            show: Show::Model(0),
            anim: None,
            colour: Some(0),
            letter: 'A',
            value: None,
            ink: 0,
            face_down: false,
            alpha: 100,
            view: View::Model,
            yaw: None,
            pitch: None,
            zoom: 100,
            turning: true,
            frozen: None,
            bare: false,
        }
    }
}

/// One control, as the page draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Item {
    Select {
        id: String,
        label: String,
        value: String,
        options: Vec<Opt>,
    },
    /// pressed or not; a click sends `send` under its id
    Toggle {
        id: String,
        label: String,
        pressed: bool,
        send: String,
    },
    /// a click sends `name` with `value`
    Button {
        id: String,
        label: String,
        name: String,
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Opt {
    pub value: String,
    pub text: String,
}

/// What the page draws over the 3D view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ui {
    pub open: bool,
    /// no controls: the 3D view alone
    pub bare: bool,
    pub heading: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sub: String,
    pub items: Vec<Item>,
}

/// The Gallery: its files, its choices and its clock.
#[derive(Debug, Clone, Default)]
pub struct Gallery {
    pub models: Vec<Model>,
    pub scenes: Vec<Scene>,
    pub font: Option<Font>,
    pub look: Option<Look>,
    text: BTreeMap<String, String>,
    pub open: bool,
    pub choice: Choice,
    /// the page's clock, microseconds
    pub clock: i64,
    /// when the animation started, and the turntable's angle since when
    pub anim_from: i64,
    pub turn_from: i64,
    pub turned: i32,
    /// the drawing buffer's size
    pub size: [i32; 2],
}

fn ok<T: Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": value }).to_string()
}

fn error(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}

fn number(key: &str, value: &str) -> Result<i32, String> {
    value
        .parse::<i32>()
        .map_err(|_| format!("The Gallery's \"{key}\" takes a whole number, not \"{value}\"."))
}

impl Gallery {
    pub fn new() -> Gallery {
        Gallery {
            size: [1, 1],
            ..Gallery::default()
        }
    }

    /// Takes the model files (one `{id: model}` object), the glyphs, the
    /// scenes (one `{id: scene}` object), the look and the Gallery's words,
    /// all checked, or says why not; nothing changes on a refusal.
    pub fn load(
        &mut self,
        models: &str,
        glyphs: &str,
        scenes: &str,
        look: &str,
        text: &str,
    ) -> Result<(), String> {
        let models = model::parse_all(models)?;
        let font = glyph::parse(glyphs)?;
        let scenes = scene::parse_all(scenes)?;
        for m in &models {
            for a in &m.anchors {
                if let Some((text, _)) = &a.text {
                    font.width(text).map_err(|c| {
                        format!("{}: the label \"{}\" has no glyph for \"{c}\".", m.id, a.name)
                    })?;
                }
            }
        }
        for s in &scenes {
            s.check(&models, &font)?;
        }
        let look = Look::parse(look)?;
        let file: FileText =
            serde_json::from_str(text).map_err(|e| format!("The Gallery's words could not be read: {e}"))?;
        let mut needed: Vec<String> = KEYS.iter().map(|k| k.to_string()).collect();
        needed.extend(PLAYER_COLOURS.iter().map(|c| format!("colour.{c}")));
        needed.extend(INKS.iter().map(|i| format!("ink.{i}")));
        needed.extend(Anim::ALL.iter().map(|a| format!("anim.{}", a.key())));
        if let Some(k) = needed
            .iter()
            .find(|k| file.strings.get(*k).is_none_or(|s| s.trim().is_empty()))
        {
            return Err(format!("The Gallery's words have no \"{k}\"."));
        }
        self.models = models;
        self.font = Some(font);
        self.scenes = scenes;
        self.look = Some(look);
        self.text = file.strings;
        self.open_model(0);
        Ok(())
    }

    fn t(&self, key: &str) -> String {
        self.text.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    /// The page's clock, in microseconds.
    pub fn advance(&mut self, now: i64) {
        self.clock = now;
    }

    /// The canvas's CSS size and pixel ratio (hundredths) in; the drawing
    /// buffer's size out.
    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_pct: i32) -> [i32; 2] {
        self.size = render::buffer_size(css_w, css_h, dpr_pct);
        self.size
    }

    /// Shows model `i` from its own angle, still, starting its clock.
    fn open_model(&mut self, i: usize) {
        let c = &mut self.choice;
        c.show = Show::Model(i);
        c.anim = None;
        c.yaw = None;
        c.pitch = None;
        c.zoom = 100;
        self.restart();
    }

    fn restart(&mut self) {
        self.anim_from = self.clock;
        self.turn_from = self.clock;
        self.turned = 0;
    }

    /// The milliseconds since the animation started, or where the clock
    /// stands stopped.
    pub fn anim_ms(&self) -> u32 {
        match self.choice.frozen {
            Some(ms) => ms,
            None => ((self.clock - self.anim_from).max(0) / 1000 % (10 * model::MAX_MS as i64)) as u32,
        }
    }

    /// The water's clock, in ms.
    fn water_ms(&self) -> u64 {
        match self.choice.frozen {
            Some(ms) => ms as u64,
            None => (self.clock.max(0) / 1000) as u64,
        }
    }

    /// The turntable's angle now.
    pub fn turn_angle(&self) -> i32 {
        let c = &self.choice;
        if !c.turning || c.frozen.is_some() {
            return self.turned;
        }
        let secs = self.look.as_ref().map_or(1, |l| l.gallery.turn_seconds) as i64;
        let more = (self.clock - self.turn_from).max(0) * TURN as i64 / (secs * 1_000_000);
        fx::wrap(self.turned + (more % TURN as i64) as i32)
    }

    fn model(&self) -> Option<&Model> {
        match self.choice.show {
            Show::Model(i) => self.models.get(i),
            Show::Scene(_) => None,
        }
    }

    /// Applies one control. A refused one changes nothing.
    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        let mut next = self.clone();
        next.apply(name, value)?;
        *self = next;
        Ok(())
    }

    fn apply(&mut self, name: &str, value: &str) -> Result<(), String> {
        let key = name
            .strip_prefix("gallery.")
            .ok_or_else(|| format!("Unknown action \"{name}\"."))?;
        if key != "open" && key != "query" && key != "close" && self.models.is_empty() {
            return Err("The Gallery has no models.".into());
        }
        match key {
            "open" => self.open = true,
            "close" => self.open = false,
            "query" => self.query(value)?,
            other => self.set(other, value)?,
        }
        Ok(())
    }

    /// Opens on the address's choices (see the module's doc).
    fn query(&mut self, query: &str) -> Result<(), String> {
        let mut opens = false;
        for pair in query.trim_start_matches('?').split('&').filter(|p| !p.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "gallery" => opens = true,
                "model" | "scene" => {
                    opens = true;
                    self.set(key, value)?;
                }
                "bare" => self.choice.bare = true,
                "ms" => {
                    let ms = number(key, value)?;
                    if !(0..=10 * model::MAX_MS as i32).contains(&ms) {
                        return Err(format!(
                            "The Gallery's \"ms\" is outside 0 to {}.",
                            10 * model::MAX_MS
                        ));
                    }
                    self.choice.frozen = Some(ms as u32);
                }
                "yaw" => self.choice.yaw = Some(deg(number(key, value)?)),
                "pitch" => {
                    let p = number(key, value)?;
                    if !(-89..=89).contains(&p) {
                        return Err("The Gallery's \"pitch\" is outside -89 to 89.".into());
                    }
                    self.choice.pitch = Some(deg(p));
                }
                "zoom" => self.zoom_to(number(key, value)?)?,
                "value" => {
                    let v = number(key, value)?;
                    if !(0..=99).contains(&v) {
                        return Err("The Gallery's \"value\" is outside 0 to 99.".into());
                    }
                    self.choice.value = Some(v);
                }
                "alpha" => {
                    let a = number(key, value)?;
                    if !(1..=100).contains(&a) {
                        return Err("The Gallery's \"alpha\" is outside 1 to 100.".into());
                    }
                    self.choice.alpha = a;
                }
                "anim" | "colour" | "letter" | "ink" | "face" | "view" | "turn" => self.set(key, value)?,
                // the rest of the page's flags
                _ => {}
            }
        }
        if opens {
            if self.models.is_empty() {
                return Err("The Gallery has no models.".into());
            }
            self.open = true;
        }
        Ok(())
    }

    fn zoom_to(&mut self, pct: i32) -> Result<(), String> {
        let [lo, hi] = self.look.as_ref().map_or([1, 1000], |l| l.gallery.zoom_pct);
        if !(lo..=hi).contains(&pct) {
            return Err(format!("The Gallery's zoom is outside {lo} to {hi}."));
        }
        self.choice.zoom = pct;
        Ok(())
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "model" => {
                let i = self
                    .models
                    .iter()
                    .position(|m| m.id == value)
                    .ok_or_else(|| format!("There is no model \"{value}\"."))?;
                self.open_model(i);
            }
            "scene" => match value {
                "" => self.open_model(0),
                _ => {
                    let i = self
                        .scenes
                        .iter()
                        .position(|s| s.id == value)
                        .ok_or_else(|| format!("There is no scene \"{value}\"."))?;
                    self.choice.show = Show::Scene(i);
                    self.choice.yaw = None;
                    self.choice.pitch = None;
                    self.choice.zoom = 100;
                    self.restart();
                }
            },
            "anim" => {
                self.choice.anim = match value {
                    "still" => None,
                    _ => {
                        let a =
                            Anim::from_key(value).ok_or_else(|| format!("There is no move \"{value}\"."))?;
                        if self.model().is_some_and(|m| m.animation(a).is_none()) {
                            return Err(format!("This model has no move \"{value}\"."));
                        }
                        Some(a)
                    }
                };
                self.anim_from = self.clock;
            }
            "colour" => {
                self.choice.colour = match value {
                    "all" => None,
                    _ => Some(
                        PLAYER_COLOURS
                            .iter()
                            .position(|c| *c == value)
                            .ok_or_else(|| format!("There is no player colour \"{value}\"."))?,
                    ),
                }
            }
            "letter" => {
                let mut chars = value.chars();
                self.choice.letter = match (chars.next(), chars.next()) {
                    (Some(c), None) if letters::value(c).is_some() => c,
                    _ => return Err(format!("There is no letter \"{value}\".")),
                };
                self.choice.value = None;
            }
            "ink" => {
                self.choice.ink = INKS
                    .iter()
                    .position(|i| *i == value)
                    .ok_or_else(|| format!("There is no ink \"{value}\"."))?
            }
            "face" => {
                self.choice.face_down = match value {
                    "up" => false,
                    "down" => true,
                    _ => return Err(format!("There is no face \"{value}\".")),
                }
            }
            "view" => {
                self.choice.view = View::ALL
                    .into_iter()
                    .find(|v| v.key() == value)
                    .ok_or_else(|| format!("There is no view \"{value}\"."))?;
                self.choice.yaw = None;
                self.choice.pitch = None;
            }
            "turn" => {
                self.turned = self.turn_angle();
                self.turn_from = self.clock;
                // a stopped clock starts again: the turntable needs it
                if self.choice.frozen.take().is_some() {
                    self.anim_from = self.clock;
                }
                self.choice.turning = match value {
                    "on" => true,
                    "off" => false,
                    _ => return Err(format!("The turn is \"on\" or \"off\", not \"{value}\".")),
                };
            }
            "zoom" => {
                let step = self.look.as_ref().map_or(20, |l| l.gallery.zoom_step_pct);
                let [lo, hi] = self.look.as_ref().map_or([1, 1000], |l| l.gallery.zoom_pct);
                let z = self.choice.zoom;
                self.choice.zoom = match value {
                    "in" => (z * (100 + step) / 100).min(hi),
                    "out" => (z * 100 / (100 + step)).max(lo),
                    _ => return Err(format!("The zoom is \"in\" or \"out\", not \"{value}\".")),
                };
            }
            other => return Err(format!("Unknown action \"gallery.{other}\".")),
        }
        Ok(())
    }

    /// The Gallery's controls.
    pub fn ui(&self) -> Ui {
        let c = &self.choice;
        let button = |id: &str, label: &str, name: &str, value: &str| Item::Button {
            id: id.into(),
            label: self.t(label),
            name: name.into(),
            value: value.into(),
        };
        let back = button("gallery.close", "back", "gallery.close", "");
        let mut ui = Ui {
            open: self.open,
            bare: c.bare,
            heading: self.t("heading"),
            sub: String::new(),
            items: Vec::new(),
        };
        if self.models.is_empty() {
            ui.sub = self.t("none");
            ui.items.push(back);
            return ui;
        }
        let opt = |value: &str, text: String| Opt {
            value: value.to_string(),
            text,
        };
        let select = |id: &str, label: &str, value: &str, options: Vec<Opt>| Item::Select {
            id: format!("gallery.{id}"),
            label: self.t(label),
            value: value.to_string(),
            options,
        };
        let model_id = self.model().map_or(String::new(), |m| m.id.clone());
        ui.items.push(select(
            "model",
            "model",
            &model_id,
            self.models.iter().map(|m| opt(&m.id, m.name.clone())).collect(),
        ));
        if !self.scenes.is_empty() {
            let mut options = vec![opt("", self.t("scene.none"))];
            options.extend(self.scenes.iter().map(|s| opt(&s.id, s.name.clone())));
            let value = match c.show {
                Show::Scene(i) => self.scenes[i].id.clone(),
                Show::Model(_) => String::new(),
            };
            ui.items.push(select("scene", "scene", &value, options));
        }
        match c.show {
            Show::Scene(i) => ui.heading = self.scenes[i].name.clone(),
            Show::Model(_) => {
                let Some(m) = self.model() else { return ui };
                ui.heading = m.name.clone();
                ui.sub = self
                    .t("size")
                    .replace("{w}", &m.width_mm().to_string())
                    .replace("{l}", &m.length_mm().to_string())
                    .replace("{h}", &m.height_mm().to_string());
                let mut moves = vec![opt("still", self.t("still"))];
                moves.extend(
                    m.animations
                        .iter()
                        .map(|a| opt(a.anim.key(), self.t(&format!("anim.{}", a.anim.key())))),
                );
                let anim = c.anim.map_or("still", |a| a.key());
                ui.items.push(select("anim", "move", anim, moves));
                if m.has_slot() {
                    let mut options: Vec<Opt> = PLAYER_COLOURS
                        .iter()
                        .map(|p| opt(p, self.t(&format!("colour.{p}"))))
                        .collect();
                    options.push(opt("all", self.t("colour.all")));
                    let value = c.colour.map_or("all", |k| PLAYER_COLOURS[k]);
                    ui.items.push(select("colour", "colour", value, options));
                }
                if m.has_label("letter") {
                    let letters = letters::VALUES
                        .iter()
                        .map(|(l, _)| opt(&l.to_string(), l.to_string()))
                        .collect();
                    ui.items
                        .push(select("letter", "letter", &c.letter.to_string(), letters));
                    let inks = INKS.iter().map(|i| opt(i, self.t(&format!("ink.{i}")))).collect();
                    ui.items.push(select("ink", "ink", INKS[c.ink], inks));
                    let faces = vec![opt("up", self.t("face.up")), opt("down", self.t("face.down"))];
                    let face = if c.face_down { "down" } else { "up" };
                    ui.items.push(select("face", "face", face, faces));
                }
                let views = View::ALL
                    .iter()
                    .map(|v| opt(v.key(), self.t(&format!("view.{}", v.key()))))
                    .collect();
                ui.items.push(select("view", "view", c.view.key(), views));
            }
        }
        let pressed = c.turning && c.frozen.is_none();
        ui.items.push(Item::Toggle {
            id: "gallery.turn".into(),
            label: self.t("turn"),
            pressed,
            send: if pressed { "off" } else { "on" }.into(),
        });
        ui.items
            .push(button("gallery.zoom.in", "zoom.in", "gallery.zoom", "in"));
        ui.items
            .push(button("gallery.zoom.out", "zoom.out", "gallery.zoom", "out"));
        ui.items.push(back);
        ui
    }

    /// The view values for a camera on `orbit`, with the fog from and to
    /// these distances.
    fn view(&self, look: &Look, orbit: &Orbit, fog: (i32, i32)) -> (Vec<i32>, V3) {
        let (eye, fwd) = orbit.eye_fwd();
        let cam = Camera {
            eye,
            fwd,
            fov: orbit.fov,
            near: (orbit.distance / 50).max(ONE / 1000),
            far: orbit.distance * 8 + fog.1,
        };
        let (light, air) = look.water.air(self.water_ms());
        (
            render::view_values(&cam, render::aspect(self.size), fog, &light, &air),
            eye,
        )
    }

    /// This frame's draw data: empty while the Gallery is closed.
    pub fn frame(&self) -> Result<Frame, String> {
        let (Some(look), Some(font)) = (self.look.as_ref(), self.font.as_ref()) else {
            return Ok(Frame::default());
        };
        if !self.open || self.models.is_empty() {
            return Ok(Frame::default());
        }
        let c = &self.choice;
        let g = &look.gallery;
        let pct = |v: i32, p: i32| (v as i64 * p as i64 / 100) as i32;
        let turned = self.turn_angle();
        let mut tris = Tris::default();
        match c.show {
            Show::Scene(i) => {
                let s = &self.scenes[i];
                s.draw(&self.models, font, look, self.anim_ms(), &mut tris)?;
                let o = s.camera;
                let orbit = Orbit {
                    yaw: c.yaw.unwrap_or(o.yaw) + turned,
                    pitch: c.pitch.unwrap_or(o.pitch),
                    distance: zoomed(o.distance, c.zoom),
                    ..o
                };
                let fog = s.fog.unwrap_or((
                    pct(orbit.distance, g.fog_pct[0]),
                    pct(orbit.distance, g.fog_pct[1]),
                ));
                let (view, eye) = self.view(look, &orbit, fog);
                Ok(Frame::new(view, tris, eye))
            }
            Show::Model(i) => {
                let m = &self.models[i];
                let poses = m.poses(c.anim, self.anim_ms());
                let transforms = m.transforms(&poses);
                let letter = c.letter;
                let value = c.value.unwrap_or_else(|| letters::value(letter).unwrap_or(0));
                let slots: Vec<usize> = match c.colour {
                    Some(k) => vec![k],
                    None if m.has_slot() => (0..PLAYER_COLOURS.len()).collect(),
                    None => vec![0],
                };
                let (centre, radius) = m.ball();
                let middle = v3(centre.x, 0, centre.z);
                let (view_yaw, view_pitch) = c.view.angles(m);
                let yaw = c.yaw.unwrap_or(view_yaw) + turned;
                let width = pct(radius * 2, ROW_GAP);
                let count = slots.len() as i32;
                for (k, slot) in slots.iter().enumerate() {
                    let across = width * (2 * k as i32 - (count - 1)) / 2;
                    // turned about its own middle, so the camera sees it
                    // from `yaw` round
                    let place = Affine::translate(v3(across, 0, 0))
                        .then(&Affine::translate(middle))
                        .then(&Affine::rot_y(-yaw))
                        .then(&Affine::translate(middle.neg()))
                        .then(&scene::placing(m, V3::ZERO, [0; 3], ONE, c.face_down));
                    let dress = Dress {
                        colour: *slot,
                        letter: Some((letter, value)),
                        ink: c.ink,
                        face_down: c.face_down,
                        alpha: c.alpha,
                    };
                    scene::draw_dressed(m, &transforms, &place, &dress, look, font, &mut tris)?;
                }
                // the camera fits the row across the screen and the model up it
                let aspect = render::aspect(self.size);
                let row_half = width * (count - 1) / 2 + radius;
                let fit = radius.max(fx::div(row_half, aspect.max(ONE)));
                let fov = deg(g.fov_deg);
                let distance = zoomed(Camera::fit(fov, fit, g.margin_pct), c.zoom);
                let orbit = Orbit {
                    target: centre,
                    yaw: 0,
                    pitch: c.pitch.unwrap_or(view_pitch),
                    distance,
                    fov,
                };
                let floor = pct(distance, g.floor_pct).max(radius * 2);
                push_floor(
                    &mut tris,
                    v3(centre.x, mm1(m.min_mm[1] - FLOOR_DROP_MM), centre.z),
                    floor,
                    Paint::lit(render::rgb(g.floor)),
                );
                let fog = (pct(distance, g.fog_pct[0]), pct(distance, g.fog_pct[1]));
                let (view, eye) = self.view(look, &orbit, fog);
                Ok(Frame::new(view, tris, eye))
            }
        }
    }

    /// `load` through the door: the Gallery's controls, or why not.
    pub fn load_json(&mut self, models: &str, glyphs: &str, scenes: &str, look: &str, text: &str) -> String {
        match self.load(models, glyphs, scenes, look, text) {
            Ok(()) => ok(&self.ui()),
            Err(e) => error(&e),
        }
    }

    /// `act` through the door: what the page draws next, or why not.
    pub fn act_json(&mut self, name: &str, value: &str) -> String {
        match self.act(name, value) {
            Ok(()) => ok(&self.ui()),
            Err(e) => error(&e),
        }
    }

    pub fn ui_json(&self) -> String {
        ok(&self.ui())
    }

    /// This frame, or an empty one (a scene that cannot be drawn was
    /// refused when it was loaded).
    pub fn frame_or_empty(&self) -> Frame {
        self.frame().unwrap_or_default()
    }
}

/// A distance `zoom` percent as near: 200 is half as far.
fn zoomed(distance: i32, zoom: i32) -> i32 {
    (distance as i64 * 100 / zoom.max(1) as i64) as i32
}

/// A flat disc of `radius` round `at`, facing up.
fn push_floor(out: &mut Tris, at: V3, radius: i32, paint: Paint) {
    let rim = |k: i32| {
        let a = TURN * k / FLOOR_SIDES;
        at.add(v3(fx::mul(fx::cos(a), radius), 0, fx::mul(fx::sin(a), radius)))
    };
    for k in 0..FLOOR_SIDES {
        out.push([at, rim(k + 1), rim(k)], V3::UP, paint);
    }
}

#[cfg(test)]
mod tests;
