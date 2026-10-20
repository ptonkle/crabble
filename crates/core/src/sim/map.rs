//! The map a game is played on: the seafloor scene (`data/scenes/seafloor.json`,
//! laid out by `tools/piles.py`) read as the game's world. Its tiles are
//! the slots where the game's tiles start (the game deals its own
//! letters); everything else but the weeds is ground ([`Ground`]).

use crate::gfx::geom::Affine;
use crate::gfx::render::{Paint, Tris};
use crate::model::Model;
use crate::scene::{self, Scene};

use super::ground::Ground;

/// The things a crab walks over without standing on them.
const NOT_GROUND: [&str; 2] = ["weed", "crab"];

/// What a game is played on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub ground: Ground,
    /// where each tile starts: its pose (the middle of its underside, its
    /// turn, face down or up)
    pub slots: Vec<Affine>,
}

impl Map {
    /// The map from a scene and the models it names, its ground `half_mm`
    /// each way from the middle.
    pub fn from_scene(s: &Scene, models: &[Model], half_mm: i32) -> Result<Map, String> {
        let mut tris = Tris::default();
        let mut slots = Vec::new();
        for t in &s.things {
            let m = models
                .iter()
                .find(|m| m.id == t.model)
                .ok_or_else(|| format!("The map names a model \"{}\" that is not loaded.", t.model))?;
            let place = scene::placing(m, t.at, t.turn, t.scale, t.dress.face_down);
            if m.kind == crate::model::Kind::Tile {
                slots.push(place);
                continue;
            }
            if NOT_GROUND.contains(&t.model.as_str()) {
                continue;
            }
            let transforms = m.transforms(&m.poses(None, 0));
            let paints = vec![Paint::lit([0; 3]); m.swatches.len()];
            m.draw(&transforms, &place, &paints, &mut tris);
        }
        Ok(Map {
            ground: Ground::from_tris(&tris.solid, half_mm),
            slots,
        })
    }

    /// Flat sand and no tiles: for tests.
    pub fn flat(half_mm: i32) -> Map {
        Map {
            ground: Ground::flat(half_mm),
            slots: Vec::new(),
        }
    }
}
