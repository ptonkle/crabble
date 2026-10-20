//! The Title: the title card and its entries. The entries are one list,
//! [`ENTRIES`], so another screen joins the Title as one more entry: add
//! its action here (its label is the string of the same key) and its arm in
//! [`App::title_entry`].
//!
//! The title card is the picked scene (CR-Q7, "standing":
//! `data/scenes/title.json`, loaded as `data.title`), drawn live behind the
//! menu with a slight idle motion ([`idle`]; its numbers are
//! `data/look.json`'s `title`). The page says whether it can draw 3D
//! (`flag.webgl`); without it, or without the card's files, the Title shows
//! the text wordmark instead (SECOND-ORDER R6.1).

use super::*;
use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::facing;
use crate::gfx::render::{self, Camera, Tris};
use crate::look::TitleLook;
use crate::model::{mm1, Kind, Model};
use crate::scene::Scene;
use view3d::{FrameInfo, Pass, PassInfo, View3d};

/// The Title's entries, top to bottom: each is an action name and the key
/// of its label.
pub const ENTRIES: &[&str] = &[
    "menu.local",
    "menu.online",
    "menu.how",
    "menu.settings",
    "menu.notes",
    "menu.gallery",
];

/// The title card's bubble model: its things rise; tiles bob; the rest
/// (the sand, the shells) stay still.
pub const BUBBLE: &str = "title-bubble";

impl App {
    /// One of the Title's entries was pressed.
    pub(super) fn title_entry(&mut self, name: &str) {
        match name {
            "menu.local" => self.go(Screen::Local),
            "menu.online" => self.go(Screen::Online),
            "menu.how" => self.go(Screen::How),
            "menu.settings" => {
                self.settings_return = self.screen == Screen::Play;
                self.go(Screen::Settings);
            }
            "menu.notes" => self.go(Screen::Notes),
            // the Gallery keeps its own core object (`crate::gallery`, its
            // own door in the shim); this only names its screen for the page
            "menu.gallery" => self.go(Screen::Gallery),
            _ => {}
        }
    }

    /// Whether the Title draws its 3D title card: the page can draw 3D
    /// and the card's files are in.
    pub fn title_card(&self) -> bool {
        self.screen == Screen::Title && self.webgl == Some(true) && self.view.title_ready()
    }

    /// The game's name, then the entries. Over the title card the name is
    /// kept for screen readers and the panel holds the card's room above
    /// the menu ("title card"); before the page has said whether it draws
    /// 3D the room is kept empty, so the wordmark does not flash up first.
    pub(super) fn title_panels(&self) -> Vec<Panel> {
        let kind = if self.title_card() || self.webgl.is_none() {
            "title card"
        } else {
            "title"
        };
        vec![
            Panel::new(kind, self.t("title"), Vec::new()),
            Panel::new(
                "menu",
                String::new(),
                ENTRIES.iter().map(|id| self.button(id)).collect(),
            ),
        ]
    }
}

/// A sine wave's value (Q16, -1 to 1) at `ms` into a cycle of `period_ms`,
/// `phase` (angle units) along.
fn wave(ms: i64, period_ms: i32, phase: i32) -> i32 {
    let p = period_ms.max(1) as i64;
    fx::sin((ms.rem_euclid(p) * fx::TURN as i64 / p) as i32 + phase)
}

/// The title card `ms` into its idle motion. Each tile bobs and rocks,
/// out of step with the one before; each bubble rises through its cycle
/// (at its own place half way through), swaying, growing in at the bottom
/// and shrinking away at the top; the camera drifts round and back.
pub fn idle(scene: &Scene, models: &[Model], t: &TitleLook, ms: i64) -> Scene {
    let mut out = scene.clone();
    let kind = |id: &str| models.iter().find(|m| m.id == id).map(|m| m.kind);
    for (k, thing) in out.things.iter_mut().enumerate() {
        // spread so neighbours are far out of step
        let phase = (k as i64 * fx::TURN as i64 * 3 / 8) as i32;
        if kind(&thing.model) == Some(Kind::Tile) {
            thing.at.y += fx::mul(wave(ms, t.bob_ms, phase), mm1(t.bob_mm));
            thing.turn[0] += fx::mul(wave(ms, t.bob_ms, phase + fx::QUARTER), deg(t.rock_deg));
        } else if thing.model == BUBBLE {
            let p = t.rise_ms.max(1) as i64;
            let at = (ms + k as i64 * p * 5 / 13).rem_euclid(p);
            // 0 to ONE through the cycle
            let f = (at * ONE as i64 / p) as i32;
            thing.at.y += fx::mul(f - ONE / 2, mm1(t.rise_mm));
            thing.at.x += fx::mul(fx::sin(2 * f + phase), mm1(t.sway_mm));
            let grow = (4 * f).min(4 * (ONE - f)).min(ONE);
            thing.scale = fx::mul(thing.scale, grow);
        }
    }
    out.camera.yaw += fx::mul(wave(ms, t.drift_ms, 0), deg(t.drift_deg));
    out
}

impl View3d {
    /// Whether the title card can be drawn: its scene, the models it was
    /// checked against, the glyphs and the look are in.
    pub fn title_ready(&self) -> bool {
        self.title.is_some() && self.font.is_some() && self.look.is_some() && !self.models.is_empty()
    }

    /// Builds the Title's frame: the title card `clock_ms` into its idle
    /// motion, one pass over the whole buffer, no static layers.
    pub fn build_title(&mut self, size: [i32; 2], clock_ms: i64) {
        self.frame.clear();
        self.info = FrameInfo::default();
        let (Some(scene), Some(look), Some(font)) = (&self.title, &self.look, &self.font) else {
            return;
        };
        let ms = clock_ms.max(0);
        let s = idle(scene, &self.models, &look.title, ms);
        let mut tris = Tris::default();
        if s.draw(&self.models, font, look, 0, &mut tris).is_err() {
            return;
        }
        let o = s.camera;
        let (eye, _) = o.eye_fwd();
        let cam = Camera {
            eye,
            // tipped down past the scene's camera, so the card sits high
            fwd: facing(o.yaw, o.pitch + deg(look.title.tilt_deg)).neg(),
            fov: o.fov,
            near: (o.distance / 50).max(ONE / 1000),
            far: o.distance * 8 + s.fog.map_or(0, |f| f.1),
        };
        let pct = |v: i32, p: i32| (v as i64 * p as i64 / 100) as i32;
        let g = &look.gallery;
        let fog = s
            .fog
            .unwrap_or((pct(o.distance, g.fog_pct[0]), pct(o.distance, g.fog_pct[1])));
        let (light, air) = look.water.air(ms as u64);
        let view = render::view_values(&cam, render::aspect(size), fog, &light, &air);
        let f = render::Frame::new(view, tris, eye);
        let viewport = [0, 0, size[0], size[1]];
        self.frame.push(Pass {
            viewport,
            layers: Vec::new(),
            view: f.view,
            solid: f.tris,
            clear: f.clear,
        });
        self.info = FrameInfo {
            layers: Vec::new(),
            passes: vec![PassInfo {
                viewport,
                layers: Vec::new(),
            }],
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::app;
    use crate::model::parse_all;
    use crate::scene;
    use crate::testkit;
    use serde_json::json;

    /// The test models: the tile, the crab and a bubble of the title kind.
    fn models_json() -> String {
        let mut bubble = testkit::tile();
        bubble["kind"] = json!("title");
        json!({ "tile": testkit::tile(), "crab": testkit::crab(), BUBBLE: bubble }).to_string()
    }

    /// A tile on the floor (thing 0) and a bubble 500 mm up (thing 1).
    fn scene_json() -> String {
        json!({
            "name": "Title",
            "camera": { "target": [0, 0, 0], "yaw": 0, "pitch": 15, "distance": 1500 },
            "things": [
                { "model": "tile", "letter": "C" },
                { "model": BUBBLE, "at": [0, 500, 0] }
            ]
        })
        .to_string()
    }

    fn loaded() -> App {
        let mut a = app();
        a.act("data.models", &models_json()).unwrap();
        a.act("data.glyphs", &testkit::font_json().to_string()).unwrap();
        a.act("data.look", &crate::look::tests::shipped()).unwrap();
        a.act("data.title", &scene_json()).unwrap();
        a
    }

    fn kind(a: &App) -> String {
        a.ui().panels[0].kind.clone()
    }

    #[test]
    fn the_title_shows_the_card_with_3d_and_the_wordmark_without() {
        // before the page says: the card's room, empty, and no 3D yet
        let mut a = loaded();
        assert_eq!(kind(&a), "title card");
        assert!(!a.ui().scene);
        // the page made its renderer: the card behind the menu
        a.act("flag.webgl", "on").unwrap();
        assert_eq!(kind(&a), "title card");
        assert!(a.ui().scene);
        assert_eq!(
            a.ui().panels[0].heading,
            "TITLE",
            "the name is kept for screen readers"
        );
        // the card is only behind the Title
        a.act("menu.local", "").unwrap();
        assert!(!a.ui().scene);
        // no WebGL 2: the wordmark
        let mut b = loaded();
        b.act("flag.webgl", "off").unwrap();
        assert_eq!(kind(&b), "title");
        assert!(!b.ui().scene);
        // WebGL but no card file: the wordmark too
        let mut c = app();
        c.act("flag.webgl", "on").unwrap();
        assert_eq!(kind(&c), "title");
        assert!(!c.ui().scene);
    }

    #[test]
    fn the_title_card_is_one_pass_over_the_whole_screen() {
        let mut a = loaded();
        a.act("flag.webgl", "on").unwrap();
        a.resize(800, 600, 100);
        a.build_frame();
        assert_eq!(a.view.info.passes.len(), 1);
        assert_eq!(a.view.info.passes[0].viewport, [0, 0, 800, 600]);
        assert!(a.view.info.layers.is_empty(), "no static layers");
        assert!(!a.view.frame[0].solid.is_empty(), "the tile is drawn");
        // off the Title, no frame
        a.act("menu.how", "").unwrap();
        a.build_frame();
        assert!(a.view.info.passes.is_empty());
    }

    #[test]
    fn idle_bobs_each_tile_and_rises_each_bubble_through_its_cycle() {
        let models = parse_all(&models_json()).unwrap();
        let s = scene::parse("title", &scene_json()).unwrap();
        let t = TitleLook {
            note: String::new(),
            tilt_deg: 0,
            bob_mm: 10,
            rock_deg: 0,
            bob_ms: 4000,
            rise_mm: 400,
            rise_ms: 1000,
            sway_mm: 0,
            drift_deg: 0,
            drift_ms: 1000,
        };
        // within a millimetre
        let near = |a: i32, mm: i32| (a as i64 * 1000 / ONE as i64 - mm as i64).abs() <= 1;
        // the tile (thing 0, in step): level at 0, 10 mm up a quarter of
        // the way through its 4 s, 10 mm down at three quarters
        assert!(near(idle(&s, &models, &t, 0).things[0].at.y, 0));
        assert!(near(idle(&s, &models, &t, 1000).things[0].at.y, 10));
        assert!(near(idle(&s, &models, &t, 3000).things[0].at.y, -10));
        // the bubble (thing 1) is 5/13 of its 1 s along at 0 (384 ms):
        // at 616 ms it starts again, 200 mm under its place and no size;
        // at 116 ms it is half way, at its own place and full size
        let start = &idle(&s, &models, &t, 616).things[1];
        assert!(near(start.at.y, 300));
        assert_eq!(start.scale, 0);
        let half = &idle(&s, &models, &t, 116).things[1];
        assert!(near(half.at.y, 500));
        assert_eq!(half.scale, ONE);
        // a quarter in (866 ms): 100 mm under, just grown in
        let quarter = &idle(&s, &models, &t, 866).things[1];
        assert!(near(quarter.at.y, 400));
        assert_eq!(quarter.scale, ONE);
    }
}
