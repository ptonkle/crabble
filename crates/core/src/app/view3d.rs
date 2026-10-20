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
//! frame, and so are the map's live things ([`live`]: what sways, such as
//! weed, kelp and fish, and what is see-through, such as the light
//! shafts), those near enough and not behind the camera.
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
use crate::scene::{self, Dress, Scene, Thing};
use crate::sim::{Crab, Gait, Map, Match, Reach, State, Tile, TileId};

use super::crab_pose::{self, Context, Extra, Parts, Rig};
/// The cosine of the half-angle of the cone round the camera's look in
/// which live things are drawn (65 degrees), Q16.
const CONE_COS: i32 = 27_697;

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
    /// the map's live things, drawn each frame ([`live`])
    live: Vec<Thing>,
    /// where each held tile was last drawn, by id: a placed tile flies
    /// from there to its square
    held_drawn: Vec<Option<Affine>>,
    /// each crab's pose memory, by seat (`app/crab_pose.rs`)
    rigs: Vec<Rig>,
    /// the stills' camera round the local crab (`?cam`, `app/fast.rs`)
    pub still_cam: Option<(i32, i32, i32)>,
    /// the frame last built
    pub frame: Vec<Pass>,
    pub info: FrameInfo,
    /// this browser's seat, as the last frame built: only its words are
    /// outlined
    me: usize,
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
        self.live.clear();
        if let (Some(s), false) = (&self.map_scene, self.models.is_empty()) {
            self.map = Some(Map::from_scene(s, &self.models, MAP_HALF_MM)?);
        }
        Ok(())
    }

    /// A new game: nothing to carry over.
    pub fn new_game(&mut self) {
        self.frame.clear();
        self.rigs.clear();
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
                let models = &self.models;
                let (live_things, still): (Vec<Thing>, Vec<Thing>) = rest.things.into_iter().partition(|t| {
                    models
                        .iter()
                        .find(|m| m.id == t.model)
                        .is_some_and(|m| live(m, t))
                });
                rest.things = still;
                self.live = live_things;
                let mut out = Tris::default();
                if rest.draw(&self.models, font, look, 0, &mut out).is_ok() {
                    self.world = out.solid;
                }
            }
        }
        &self.world
    }

    /// The map's live things `ms` into their animations, but those
    /// further than `reach` from `eye` or wholly behind it (looking along
    /// `fwd`).
    fn draw_live(&self, eye: V3, fwd: V3, reach: i32, ms: u32, out: &mut Tris) {
        let (Some(look), Some(font)) = (&self.look, &self.font) else {
            return;
        };
        for t in &self.live {
            let Some(m) = self.model(&t.model) else {
                continue;
            };
            // how far the model reaches from where it stands, scaled
            let size = m
                .min_mm
                .iter()
                .chain(&m.max_mm)
                .map(|v| v.abs())
                .max()
                .unwrap_or(0);
            let r = fx::mul(mm1(size), t.scale) * 2;
            let d = t.at.sub(eye);
            // outside a cone 65 degrees round the look, wider than the
            // view's widest (about 46 degrees at 16:9), by its size
            if d.longer_than(reach.saturating_add(r)) || d.dot(fwd) < fx::mul(d.len(), CONE_COS) - r {
                continue;
            }
            let poses = m.poses(t.anim.map(|a| a.0), t.anim.map_or(0, |a| a.1.wrapping_add(ms)));
            let transforms = m.transforms(&poses);
            let place = scene::placing(m, t.at, t.turn, t.scale, t.dress.face_down);
            let _ = scene::draw_dressed(m, &transforms, &place, &t.dress, look, font, out);
        }
    }

    /// Draws a tile at rest where it lies.
    fn draw_tile(&self, t: &Tile, place: &Affine, scale_pct: i32, out: &mut Tris) {
        self.draw_tile_ink(t, place, scale_pct, 0, out)
    }

    /// Draws a tile in an ink of `INKS` (plain, or a real word's green).
    fn draw_tile_ink(&self, t: &Tile, place: &Affine, scale_pct: i32, ink: usize, out: &mut Tris) {
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
            ink,
            ..Default::default()
        };
        let _ = scene::draw_dressed(m, &transforms, &place, &dress, look, font, out);
    }

    /// The word marks of the board as it is: the outlines of seat `me`'s
    /// words only, dashed for words that are not real (the person, F8:
    /// "you don't need to keep track of others words"; R7.1).
    fn marks(game: &Match, me: usize) -> crate::marks::Marks {
        crate::marks::Marks {
            me: Some(me),
            words: game.words().into_iter().map(|w| w.word).collect(),
            ghost: None,
        }
    }

    /// Draws the ghost tile at a drawing pose: blank, whatever is held
    /// (the person, F10: "just have it be blank always"; R7.4).
    fn draw_ghost(&self, place: &Affine, out: &mut Tris) {
        let (Some(m), Some(look), Some(font)) = (self.model("ghost-tile"), &self.look, &self.font) else {
            return;
        };
        let transforms = m.transforms(&m.poses(None, 0));
        let dress = ghost_dress();
        let _ = scene::draw_dressed(m, &transforms, place, &dress, look, font, out);
    }

    /// How long a tile takes to fly to a claw or a square (ms).
    fn change_ms(&self) -> i64 {
        self.look.as_ref().map_or(1, |l| l.held.change_ms as i64)
    }

    /// Whether a tile is drawn in a static layer `now`: at rest, loose or
    /// placed, and not still flying to its square.
    fn at_rest(&self, t: &Tile, now: i64) -> bool {
        match t.state {
            State::Loose { awake: false } => true,
            State::Placed { .. } => now - t.since_ms >= self.change_ms() || t.since_ms <= 0,
            _ => false,
        }
    }

    /// Each static layer's signature: it changes when what the layer
    /// draws changes.
    fn signatures(&self, game: &Match) -> Vec<String> {
        let mut out = vec![format!("world-{}", self.models.len())];
        let now = game.play_ms();
        for chunk in game.tiles.all.chunks(BUCKET) {
            let mut h = 0u64;
            for t in chunk {
                let p = &t.pose;
                let parts: [i64; 7] = [
                    t.id as i64,
                    self.at_rest(t, now) as i64,
                    matches!(t.state, State::Placed { .. }) as i64,
                    p.t.x as i64,
                    p.t.y as i64,
                    p.t.z as i64 ^ ((p.c1.y as i64) << 20),
                    p.c0.x as i64 ^ ((t.face_down as i64) << 40),
                ];
                for v in parts {
                    h = mix(h ^ v as u64);
                }
            }
            // a placement anywhere can turn a placed letter green
            if chunk.iter().any(|t| matches!(t.state, State::Placed { .. })) {
                h = mix(h ^ game.placements as u64);
            }
            out.push(format!("{h:016x}"));
        }
        out.push(format!("marks-{}-{}", self.me, game.placements));
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
        let marks = Self::marks(g, self.me);
        if k == 1 + g.tiles.all.len().div_ceil(BUCKET) {
            if let Some(look) = &self.look {
                crate::marks::draw(&marks, look, &mut out);
            }
            return out.solid;
        }
        let green = crate::marks::real_cells(&marks.words);
        let now = g.play_ms();
        for t in g.tiles.all.chunks(BUCKET).nth(k - 1).unwrap_or(&[]) {
            if self.at_rest(t, now) {
                let ink = match t.state {
                    State::Placed { square, .. } if green.contains(&square) => 1,
                    _ => 0,
                };
                self.draw_tile_ink(t, &t.draw_pose(), 100, ink, &mut out);
            }
        }
        out.solid
    }

    /// A held tile's drawing pose in claw `claw` of a crab posed by
    /// `transforms` and placed by `place`: at half size (shrinking from
    /// full over the change), face up, tipped back toward the camera, set
    /// out to its side; flying in from where it lay; and lifted clear of
    /// anything under it (the board's rim beside a crab).
    fn held_pose(
        &self,
        game: &Match,
        m: &Model,
        transforms: &[Affine],
        place: &Affine,
        t: &Tile,
        claw: usize,
    ) -> Affine {
        let (Some(look), Some(tm)) = (&self.look, self.model("tile")) else {
            return t.draw_pose();
        };
        let h = &look.held;
        let since = game.play_ms() - t.since_ms;
        let Some(at) = in_claw(
            look,
            tm,
            (m, transforms, place),
            claw,
            h.tile_pct_at(true, since),
            fx::HALF,
        ) else {
            return t.draw_pose();
        };
        let flown = blend(&t.from, &at, since, self.change_ms());
        clear_of(game, flown, t.id)
    }

    /// Poses a crab: its walk, jump or stance, its claws raised round what
    /// they hold, its last grab or place, its extra motion (`rig`,
    /// `app/crab_pose.rs`), and the tiles in its claws. `yaw` is the way it
    /// faces. Returns where each held tile was drawn.
    fn draw_crab(
        &self,
        game: &Match,
        i: usize,
        c: &Crab,
        yaw: i32,
        rig: &mut Rig,
        out: &mut Tris,
    ) -> Vec<(TileId, Affine)> {
        let (Some(m), Some(look), Some(font)) = (self.model("crab"), &self.look, &self.font) else {
            return Vec::new();
        };
        let now = game.play_ms().max(0) as u32;
        let body = if !c.on_ground {
            (
                Anim::Jump,
                c.air_ms * anim_ms(m, Anim::Jump) / jump_air_ms(game).max(1),
            )
        } else {
            gait_body(m, c.gait, c.walk_ms, now)
        };
        let cx = Context {
            now: game.play_ms(),
            seat: i,
            speed: game.rules.speed,
            jump_speed: game.rules.jump_speed,
            cycle_ms: anim_ms(
                m,
                if matches!(c.gait, Gait::Forward | Gait::Back) {
                    Anim::WalkFwd
                } else {
                    Anim::WalkSide
                },
            ),
        };
        rig.update(c, yaw, &cx, &look.crab);
        let doing = Doing {
            body,
            now,
            holds: [c.claws[0].is_some(), c.claws[1].is_some()],
            reach: c
                .reach
                .map(|r| r.map(|(reach, at)| (reach, (game.play_ms() - at).max(0) as u32))),
            extra: Some(Extra {
                motion: rig.motion(c, yaw, &cx),
                look: &look.crab,
            }),
        };
        let transforms = pose_crab(m, &doing);
        let place = Affine::translate(c.pos).then(&Affine::rot_y(yaw));
        let dress = Dress {
            colour: i % crate::look::PLAYER_COLOURS.len(),
            ..Default::default()
        };
        let _ = scene::draw_dressed(m, &transforms, &place, &dress, look, font, out);
        let mut held = Vec::new();
        for claw in 0..2 {
            let Some(t) = c.claws[claw].and_then(|id| game.tiles.get(id)) else {
                continue;
            };
            let pose = self.held_pose(game, m, &transforms, &place, t, claw);
            self.draw_tile(t, &pose, 100, out);
            held.push((t.id, pose));
        }
        held
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
        self.me = me;
        let Some(look) = self.look.clone() else {
            return;
        };
        let p = &look.play;
        let (light, air) = look.water.air(clock_ms.max(0) as u64);
        // the world, the tile buckets and the word marks
        let layers: Vec<usize> = (0..=g.tiles.all.len().div_ceil(BUCKET) + 1).collect();

        // the main view, behind this browser's crab
        let mut tris = Tris::default();
        let mut held = Vec::new();
        let mut rigs = std::mem::take(&mut self.rigs);
        rigs.resize(g.crabs.len(), Rig::default());
        for (i, c) in g.crabs.iter().enumerate() {
            let face = if i == me { yaw } else { c.yaw };
            held.extend(self.draw_crab(g, i, c, face, &mut rigs[i], &mut tris));
        }
        self.rigs = rigs;
        if self.held_drawn.len() != g.tiles.all.len() {
            self.held_drawn = vec![None; g.tiles.all.len()];
        }
        for (id, pose) in held {
            self.held_drawn[id as usize] = Some(pose);
        }
        let now = g.play_ms();
        for t in &g.tiles.all {
            match t.state {
                State::Loose { awake: true } => self.draw_tile(t, &t.draw_pose(), 100, &mut tris),
                // flying from the claw to its square, growing back to full
                State::Placed { .. } if !self.at_rest(t, now) => {
                    let from = self.held_drawn[t.id as usize].unwrap_or(t.from);
                    let pose = blend(&from, &t.draw_pose(), now - t.since_ms, self.change_ms());
                    self.draw_tile(t, &pose, 100, &mut tris);
                }
                _ => {}
            }
        }
        // the ghost tile where a click would place it, and its ring
        if let Some((_, _, sq)) = g.ghost(me) {
            let pose = crate::sim::claims::square_pose(&g.rules, sq);
            self.draw_ghost(&pose, &mut tris);
            let marks = crate::marks::Marks {
                me: None,
                words: Vec::new(),
                ghost: Some(sq),
            };
            crate::marks::draw(&marks, &look, &mut tris);
        }
        // the loose tile a free claw would grab, ringed (F6)
        if let Some(t) = g.grabbable(me).and_then(|id| g.tiles.get(id)) {
            crate::marks::draw_grab(&look, &t.pose, &mut tris);
        }
        let me_crab = &g.crabs[me];
        let (mut eye, mut fwd) = g.camera_at(me_crab.pos, yaw, pitch);
        if let Some((round, up, mm)) = self.still_cam {
            let target = me_crab.pos.add(v3(0, mm1(25), 0));
            fwd = facing(yaw + deg(round), -deg(up));
            eye = target.sub(fwd.scale(mm1(mm)));
        }
        let cam = Camera {
            eye,
            fwd,
            fov: deg(p.fov_deg),
            near: mm1(p.near_mm),
            far: mm1(p.far_mm),
        };
        self.world();
        self.draw_live(eye, fwd, mm1(p.live_mm), clock_ms.max(0) as u32, &mut tris);
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

/// Whether a thing of the map is drawn each frame rather than in the
/// static world: it plays an animation (weed, kelp, an anemone, fish), or
/// has see-through colours (a light shaft), which the static layers,
/// being solid, leave out.
pub fn live(m: &Model, t: &Thing) -> bool {
    t.anim.is_some() || m.swatches.iter().any(|s| s.alpha < 100)
}

/// What a crab is doing, for posing it ([`pose_crab`]): its body's move
/// and how far into it, `now` (for the moves that loop on the clock),
/// which claws hold a tile, and each claw's last grab or place with how
/// long ago it began. The game's crabs and the Title's (`app/title_life.rs`)
/// are both posed from one of these.
pub struct Doing<'a> {
    pub body: (Anim, u32),
    pub now: u32,
    pub holds: [bool; 2],
    pub reach: [Option<(Reach, u32)>; 2],
    /// the motion past the keyed moves (`app/crab_pose.rs`): the bob, lean,
    /// squash, eyes, claws and fidget; none for the keyed moves alone
    pub extra: Option<Extra<'a>>,
}

/// A move's length in the model, ms (1 when it has none).
pub fn anim_ms(m: &Model, a: Anim) -> u32 {
    m.animation(a).map_or(1, |x| x.ms.max(1))
}

/// The body's move for a crab on the ground walking `gait`, `walk_ms` of
/// walking in: a walk played backward for the other way.
pub fn gait_body(m: &Model, gait: Gait, walk_ms: u32, now: u32) -> (Anim, u32) {
    let back = |a: Anim, ms: u32| anim_ms(m, a) - 1 - ms % anim_ms(m, a);
    match gait {
        Gait::Still => (Anim::Idle, now),
        Gait::Forward => (Anim::WalkFwd, walk_ms),
        Gait::Back => (Anim::WalkFwd, back(Anim::WalkFwd, walk_ms)),
        Gait::Left => (Anim::WalkSide, walk_ms),
        Gait::Right => (Anim::WalkSide, back(Anim::WalkSide, walk_ms)),
    }
}

/// The crab model's part transforms for what it is doing: the body's
/// move, the claws raised round what they hold, and a grab or place still
/// under way over them.
pub fn pose_crab(m: &Model, d: &Doing) -> Vec<Affine> {
    let mut layers = vec![d.body];
    match (d.holds[0], d.holds[1]) {
        (true, true) => layers.push((Anim::Hold, d.now)),
        (true, false) => layers.push((Anim::HoldLeft, d.now)),
        (false, true) => layers.push((Anim::HoldRight, d.now)),
        _ => {}
    }
    for (claw, (grab, put)) in [
        (Anim::GrabLeft, Anim::PlaceLeft),
        (Anim::GrabRight, Anim::PlaceRight),
    ]
    .into_iter()
    .enumerate()
    {
        if let Some((reach, since)) = d.reach[claw] {
            let anim = if reach == Reach::Grab { grab } else { put };
            if since < anim_ms(m, anim) {
                layers.push((anim, since));
            }
        }
    }
    let mut poses = m.layered(&layers);
    let squash = d
        .extra
        .as_ref()
        .map(|x| crab_pose::dress(&x.motion, d.holds, d.reach, &mut poses, &Parts::of(m), x.look));
    let mut transforms = m.transforms(&poses);
    if let Some(s) = squash {
        crab_pose::squashed(&mut transforms, s);
    }
    transforms
}

/// Where a tile held in claw `claw` of a crab posed by `transforms` and
/// placed by `place` is drawn, at `pct` of its size: face up, tipped back
/// toward the camera, set out to its side. `turn` turns its letter about
/// the up axis: half a turn faces it back, to the game's camera behind the
/// crab; none faces it ahead, to the Title's in front. `None` without the
/// anchor.
pub fn in_claw(
    look: &Look,
    tm: &Model,
    (m, transforms, place): (&Model, &[Affine], &Affine),
    claw: usize,
    pct: i32,
    turn: i32,
) -> Option<Affine> {
    let h = &look.held;
    let (anchor, side) = if claw == crate::sim::tiles::LEFT {
        ("claw_left", h.spread_mm)
    } else {
        ("claw_right", -h.spread_mm)
    };
    let a = m.anchor(anchor)?;
    let frame = place.then(&m.anchor_frame(a, transforms));
    let local = scene::placing(
        tm,
        crate::model::mm([side, 0, 0]),
        [deg(h.tilt_deg), turn, 0],
        pct * ONE / 100,
        false,
    );
    Some(frame.then(&local))
}

/// Between two poses, `ms` of `span` along: the place, then each axis,
/// eased; a turn that cannot be blended (half a turn over) snaps half way.
/// How the ghost tile is dressed: whole, and with no letter.
fn ghost_dress() -> Dress {
    Dress {
        letter: None,
        alpha: 100,
        ..Default::default()
    }
}

#[cfg(test)]
#[test]
fn the_ghost_tile_carries_no_letter() {
    let d = ghost_dress();
    assert_eq!((d.letter, d.alpha), (None, 100));
}

pub fn blend(from: &Affine, to: &Affine, ms: i64, span: i64) -> Affine {
    if ms >= span || span <= 0 {
        return *to;
    }
    let s = fx::ratio(ms.max(0) as i32, span as i32);
    let s = 3 * fx::mul(s, s) - 2 * fx::mul(fx::mul(s, s), s);
    let lerp = |a: V3, b: V3| a.lerp(b, s);
    // a scaled frame keeps its size through the blend
    let size = |a: &Affine| a.c0.len();
    let k = fx::lerp(size(from), size(to), s);
    let c1 = lerp(from.c1.norm(), to.c1.norm()).norm();
    let c0 = lerp(from.c0.norm(), to.c0.norm());
    let c0 = c0.sub(c1.scale(c0.dot(c1))).norm();
    if c1 == V3::ZERO || c0 == V3::ZERO {
        return if s < ONE / 2 { *from } else { *to };
    }
    let c2 = c0.cross(c1).norm();
    Affine {
        c0: c0.scale(k),
        c1: c1.scale(k),
        c2: c2.scale(k),
        t: lerp(from.t, to.t),
    }
}

/// A held tile's pose lifted until no point of its underside is under the
/// ground or a tile at rest beneath it.
fn clear_of(g: &Match, pose: Affine, id: TileId) -> Affine {
    let mut lift = 0;
    for p in crate::sim::tiles::underside_of(&pose) {
        let s = g.tiles.support(&g.ground, p.x, p.z, p.y, id) + mm1(2);
        lift = lift.max(s - p.y);
    }
    Affine::translate(v3(0, lift, 0)).then(&pose)
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
