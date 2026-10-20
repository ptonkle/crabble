//! The game's 3D view (PLAN.md, gameplay 12): the core poses everything
//! and the page replays it through `web/gl.js`.
//!
//! A frame is a list of passes, each a viewport (in the drawing buffer's
//! pixels, from its bottom-left corner), its view values, the static
//! layers it draws and its own moving triangles: the main view behind the
//! crab, then the overhead view in the corner. Static layers are the world
//! (seafloor, board, mounds and props) and the tiles at rest, in buckets
//! of [`BUCKET`] by id; each carries a signature, and the page asks for a
//! layer's triangles again only when its signature changes. The crabs,
//! held and tumbling tiles and the overhead view's circles are drawn every
//! frame.
//!
//! The numbers for drawing are `data/look.json` (`play`, `held`).

use serde::Serialize;

use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::{facing, v3, Affine, V3};
use crate::gfx::render::{self, Air, Camera, Paint, Tris};
use crate::glyph::Font;
use crate::look::Look;
use crate::model::{self, mm1, Anim, Kind, Model};
use crate::rng::mix;
use crate::scene::{self, Dress, Scene};
use crate::sim::{Crab, Gait, Map, Match, State, Tile};

/// Tiles to a static layer.
pub const BUCKET: usize = 30;

/// How far the map's ground reaches from the middle, each way (mm): past
/// where a crab may walk.
pub const MAP_HALF_MM: i32 = 6000;

/// The overhead view's look straight down, short of exactly down so the
/// camera keeps a way up (the board's far edge at the top).
const DOWN: i32 = fx::QUARTER - fx::TURN / 360;

/// One pass of a frame: see the module's doc.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pass {
    pub viewport: [i32; 4],
    pub layers: Vec<usize>,
    pub view: Vec<i32>,
    pub solid: Vec<i32>,
    pub clear: Vec<i32>,
}

/// What the page needs each frame to know what to draw: each static
/// layer's signature, and each pass's viewport and layers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FrameInfo {
    pub layers: Vec<String>,
    pub passes: Vec<PassInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PassInfo {
    pub viewport: [i32; 4],
    pub layers: Vec<usize>,
}

/// The 3D view's files and what is built from them.
#[derive(Default)]
pub struct View3d {
    pub models: Vec<Model>,
    pub font: Option<Font>,
    pub look: Option<Look>,
    pub map_scene: Option<Scene>,
    /// the map, once its scene and models are both in
    pub map: Option<Map>,
    /// the title card (`data.title`, `app/title.rs`)
    pub title: Option<Scene>,
    /// the world's triangles, built once
    world: Vec<i32>,
    /// the frame last built
    pub frame: Vec<Pass>,
    pub info: FrameInfo,
}

impl View3d {
    pub fn load(&mut self, name: &str, value: &str) -> Result<(), String> {
        if name == "data.title" {
            let scene = scene::parse("title", value)?;
            if let Some(font) = &self.font {
                scene.check(&self.models, font)?;
            }
            self.title = Some(scene);
            return Ok(());
        }
        match name {
            "data.models" => self.models = model::parse_all(value)?,
            "data.glyphs" => self.font = Some(crate::glyph::parse(value)?),
            "data.look" => self.look = Some(Look::parse(value)?),
            "data.map" => self.map_scene = Some(scene::parse("seafloor", value)?),
            _ => return Err(format!("Unknown file \"{name}\".")),
        }
        self.world.clear();
        if let (Some(s), false) = (&self.map_scene, self.models.is_empty()) {
            self.map = Some(Map::from_scene(s, &self.models, MAP_HALF_MM)?);
        }
        Ok(())
    }

    /// A new game: nothing to carry over.
    pub fn new_game(&mut self) {
        self.frame.clear();
    }

    fn model(&self, id: &str) -> Option<&Model> {
        self.models.iter().find(|m| m.id == id)
    }

    fn ready(&self) -> bool {
        self.font.is_some()
            && self.look.is_some()
            && self.model("crab").is_some()
            && self.model("tile").is_some()
    }

    /// The world's triangles: every thing of the map but its tiles.
    fn world(&mut self) -> &[i32] {
        if self.world.is_empty() {
            if let (Some(s), Some(look), Some(font)) = (&self.map_scene, &self.look, &self.font) {
                let mut rest = s.clone();
                let tiles: Vec<&str> = self
                    .models
                    .iter()
                    .filter(|m| m.kind == Kind::Tile)
                    .map(|m| m.id.as_str())
                    .collect();
                rest.things.retain(|t| !tiles.contains(&t.model.as_str()));
                let mut out = Tris::default();
                if rest.draw(&self.models, font, look, 0, &mut out).is_ok() {
                    self.world = out.solid;
                }
            }
        }
        &self.world
    }

    /// Draws a tile at rest where it lies.
    fn draw_tile(&self, t: &Tile, place: &Affine, scale_pct: i32, out: &mut Tris) {
        let (Some(m), Some(look), Some(font)) = (self.model("tile"), &self.look, &self.font) else {
            return;
        };
        let transforms = m.transforms(&m.poses(None, 0));
        let place = place.then(&Affine::scale(v3(
            scale_pct * ONE / 100,
            scale_pct * ONE / 100,
            scale_pct * ONE / 100,
        )));
        let dress = Dress {
            letter: Some((t.letter, t.value)),
            ..Default::default()
        };
        let _ = scene::draw_dressed(m, &transforms, &place, &dress, look, font, out);
    }

    /// Whether a tile is drawn in a static layer: at rest, loose or placed.
    fn at_rest(t: &Tile) -> bool {
        matches!(t.state, State::Loose { awake: false } | State::Placed { .. })
    }

    /// Each static layer's signature: it changes when what the layer
    /// draws changes.
    fn signatures(&self, game: &Match) -> Vec<String> {
        let mut out = vec![format!("world-{}", self.models.len())];
        for chunk in game.tiles.all.chunks(BUCKET) {
            let mut h = 0u64;
            for t in chunk {
                let p = &t.pose;
                let parts: [i64; 6] = [
                    t.id as i64,
                    Self::at_rest(t) as i64,
                    matches!(t.state, State::Placed { .. }) as i64,
                    p.t.x as i64,
                    p.t.y as i64,
                    p.t.z as i64 ^ ((p.c1.y as i64) << 20),
                ];
                for v in parts {
                    h = mix(h ^ v as u64);
                }
            }
            out.push(format!("{h:016x}"));
        }
        out
    }

    /// Layer `k`'s triangles: the world, or bucket `k - 1` of the tiles.
    pub fn layer(&mut self, game: Option<&Match>, k: usize) -> Vec<i32> {
        if k == 0 {
            return self.world().to_vec();
        }
        let Some(g) = game else {
            return Vec::new();
        };
        let mut out = Tris::default();
        for t in g.tiles.all.chunks(BUCKET).nth(k - 1).unwrap_or(&[]) {
            if Self::at_rest(t) {
                self.draw_tile(t, &t.pose, 100, &mut out);
            }
        }
        out.solid
    }

    /// Poses a crab: its walk, jump or stance, its claws raised round what
    /// they hold, and the tiles in them. `yaw` is the way it faces.
    fn draw_crab(&self, game: &Match, i: usize, c: &Crab, yaw: i32, out: &mut Tris) {
        let (Some(m), Some(look), Some(font)) = (self.model("crab"), &self.look, &self.font) else {
            return;
        };
        let now = game.play_ms().max(0) as u32;
        let len = |a: Anim| m.animation(a).map_or(1, |x| x.ms.max(1));
        let back = |a: Anim, ms: u32| len(a) - 1 - ms % len(a);
        let body = if !c.on_ground {
            (Anim::Jump, c.air_ms * len(Anim::Jump) / jump_air_ms(game).max(1))
        } else {
            match c.gait {
                Gait::Still => (Anim::Idle, now),
                Gait::Forward => (Anim::WalkFwd, c.walk_ms),
                Gait::Back => (Anim::WalkFwd, back(Anim::WalkFwd, c.walk_ms)),
                Gait::Left => (Anim::WalkSide, c.walk_ms),
                Gait::Right => (Anim::WalkSide, back(Anim::WalkSide, c.walk_ms)),
            }
        };
        let mut layers = vec![body];
        match (c.claws[0].is_some(), c.claws[1].is_some()) {
            (true, true) => layers.push((Anim::Hold, now)),
            (true, false) => layers.push((Anim::HoldLeft, now)),
            (false, true) => layers.push((Anim::HoldRight, now)),
            _ => {}
        }
        let transforms = m.transforms(&m.layered(&layers));
        let place = Affine::translate(c.pos).then(&Affine::rot_y(yaw));
        let dress = Dress {
            colour: i % crate::look::PLAYER_COLOURS.len(),
            ..Default::default()
        };
        let _ = scene::draw_dressed(m, &transforms, &place, &dress, look, font, out);
    }

    /// Builds this frame for a buffer of `size` pixels, the page's clock
    /// `clock_ms` in (for the water's ripples).
    pub fn build(
        &mut self,
        game: Option<&Match>,
        local: Option<(usize, i32, i32)>,
        overhead_big: bool,
        size: [i32; 2],
        clock_ms: i64,
    ) {
        self.frame.clear();
        self.info = FrameInfo::default();
        let (Some(g), Some((me, yaw, pitch))) = (game, local) else {
            return;
        };
        if !self.ready() || me >= g.crabs.len() {
            return;
        }
        let Some(look) = self.look.clone() else {
            return;
        };
        let p = &look.play;
        let (light, air) = look.water.air(clock_ms.max(0) as u64);
        let layers: Vec<usize> = (0..=g.tiles.all.len().div_ceil(BUCKET)).collect();

        // the main view, behind this browser's crab
        let mut tris = Tris::default();
        for (i, c) in g.crabs.iter().enumerate() {
            let face = if i == me { yaw } else { c.yaw };
            self.draw_crab(g, i, c, face, &mut tris);
        }
        for t in &g.tiles.all {
            if matches!(t.state, State::Loose { awake: true }) {
                self.draw_tile(t, &t.pose, 100, &mut tris);
            }
        }
        let me_crab = &g.crabs[me];
        let (eye, fwd) = g.camera_at(me_crab.pos, yaw, pitch);
        let cam = Camera {
            eye,
            fwd,
            fov: deg(p.fov_deg),
            near: mm1(p.near_mm),
            far: mm1(p.far_mm),
        };
        let view = render::view_values(
            &cam,
            render::aspect(size),
            (mm1(p.fog_mm[0]), mm1(p.fog_mm[1])),
            &light,
            &air,
        );
        let f = render::Frame::new(view, tris, eye);
        self.frame.push(Pass {
            viewport: [0, 0, size[0], size[1]],
            layers: layers.clone(),
            view: f.view,
            solid: f.tris,
            clear: f.clear,
        });

        // the overhead view: the board from straight above
        let o = &p.overhead;
        let (side, at) = if overhead_big {
            let s = size[0].min(size[1]) * o.big_pct / 100;
            (s, [(size[0] - s) / 2, (size[1] - s) / 2])
        } else {
            let s = size[1] * o.size_pct / 100;
            let gap = size[1] * o.margin_pct / 100;
            (s, [gap, gap])
        };
        let board_half = self
            .model("board")
            .map_or(g.rules.board_half, |b| mm1(b.max_mm[0]));
        let half = (board_half as i64 * o.room_pct as i64 / 100) as i32;
        let fov = deg(o.fov_deg);
        let distance = fx::div(half, fx::div(fx::sin(fov / 2), fx::cos(fov / 2)).max(1));
        let top = v3(0, g.rules.board_top, 0);
        let down = facing(fx::HALF, -DOWN);
        let eye = top.sub(down.scale(distance));
        let cam = Camera {
            eye,
            fwd: down,
            fov,
            near: distance / 2,
            far: distance * 2,
        };
        let mut tris = Tris::default();
        let lift = mm1(o.lift_mm);
        for (i, c) in g.crabs.iter().enumerate() {
            let colour = look.player(i % crate::look::PLAYER_COLOURS.len());
            let paint = Paint {
                rgb: colour,
                glow: ONE,
                alpha: o.alpha_pct * ONE / 100,
            };
            disc(
                &mut tris,
                v3(c.pos.x, top.y + lift + i as i32 * mm1(2), c.pos.z),
                mm1(o.circle_mm / 2),
                o.sides,
                paint,
            );
        }
        let far = distance * 4;
        let view = render::view_values(&cam, ONE, (far, far * 2), &light, &Air::plain(&light));
        let f = render::Frame::new(view, tris, eye);
        self.frame.push(Pass {
            viewport: [at[0], at[1], side, side],
            layers,
            view: f.view,
            solid: f.tris,
            clear: f.clear,
        });

        self.info = FrameInfo {
            layers: self.signatures(g),
            passes: self
                .frame
                .iter()
                .map(|p| PassInfo {
                    viewport: p.viewport,
                    layers: p.layers.clone(),
                })
                .collect(),
        };
    }
}

/// The jump's time in the air, ms, from its rules.
fn jump_air_ms(g: &Match) -> u32 {
    // up and down at the same speed: twice the speed over the pull
    (2 * g.rules.jump_speed as i64 * 1000 / g.rules.gravity.max(1) as i64) as u32
}

/// A flat disc facing up: `sides` edges round `at`.
fn disc(out: &mut Tris, at: V3, radius: i32, sides: i32, paint: Paint) {
    let ring: Vec<V3> = (0..sides)
        .map(|k| {
            let a = fx::TURN * k / sides;
            at.add(v3(fx::mul(fx::cos(a), radius), 0, fx::mul(fx::sin(a), radius)))
        })
        .collect();
    out.push_poly(&ring, v3(0, ONE, 0), paint);
}
