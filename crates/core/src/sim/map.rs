//! The map a game is played on: the seafloor scene (`data/scenes/seafloor.json`,
//! laid out by `tools/piles.py`) read as the game's world. Its tiles are
//! the slots where the game's tiles start (the game deals its own
//! letters); everything else but what sways (weed, kelp, anemones, fish:
//! [`passes_through`]) is ground ([`Ground`]).

use crate::gfx::geom::Affine;
use crate::gfx::render::{Paint, Tris};
use crate::model::Model;
use crate::scene::{self, Scene};

use super::ground::Ground;
use super::tiles::{Physics, State, Tile, TileId, Tiles};

/// Whether a crab passes through this model rather than standing on it:
/// a crab, or anything that sways in the current (weed, kelp, an
/// anemone's tentacles, a school of fish). See-through faces are never
/// ground either: the ground is built from solid faces only.
pub fn passes_through(m: &Model) -> bool {
    m.id == "crab" || m.animation(crate::model::Anim::Sway).is_some()
}

/// What a game is played on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub ground: Ground,
    /// where each tile starts: its box's frame (`tiles::Tile::pose`) and
    /// whether it lies face down
    pub slots: Vec<(Affine, bool)>,
    /// whether the slots have been settled by the game's own tumbling
    /// ([`Map::settle`])
    pub settled: bool,
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
                // the box's frame: the turn without the turning over
                let frame = scene::placing(m, t.at, t.turn, t.scale, false);
                slots.push((frame, t.dress.face_down));
                continue;
            }
            if passes_through(m) {
                continue;
            }
            let transforms = m.transforms(&m.poses(None, 0));
            // each swatch as solid as it is drawn, so a see-through face (a
            // light shaft) is never ground (R16.11)
            let paints: Vec<Paint> = m
                .swatches
                .iter()
                .map(|w| Paint {
                    alpha: w.alpha,
                    ..Paint::lit([0; 3])
                })
                .collect();
            m.draw(&transforms, &place, &paints, &mut tris);
        }
        Ok(Map {
            ground: Ground::from_tris(&tris.solid, half_mm),
            slots,
            settled: false,
        })
    }

    /// Flat sand and no tiles: for tests.
    pub fn flat(half_mm: i32) -> Map {
        Map {
            ground: Ground::flat(half_mm),
            slots: Vec::new(),
            settled: false,
        }
    }

    /// Settles the slots by the game's own tumbling (SECOND-ORDER R16.4):
    /// `tools/piles.py` lays the piles out by a lighter rule of its own, so
    /// some of its tiles lean past what holds them or hover a little. Every
    /// slot is woken at once and run, `tick_ms` a tick, until all sleep (at
    /// most 20 s), and each slot takes the pose it came to rest in. The same
    /// on every machine, so every browser starts from the same piles.
    pub fn settle(&mut self, ph: &Physics, tick_ms: i32) {
        let mut ts = Tiles {
            all: self
                .slots
                .iter()
                .enumerate()
                .map(|(i, (pose, down))| {
                    let mut t = Tile::new(i as TileId, 'A', 1, *pose, *down);
                    t.state = State::Loose { awake: true };
                    t
                })
                .collect(),
        };
        ts.find_rests();
        let ticks = 20_000 / tick_ms.max(1);
        for _ in 0..ticks {
            if !ts.any_awake() {
                break;
            }
            ts.step(&self.ground, ph, tick_ms);
        }
        for (slot, t) in self.slots.iter_mut().zip(&ts.all) {
            *slot = (t.pose, t.face_down);
        }
        self.settled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::geom::v3;
    use crate::model::mm1;

    #[test]
    fn settling_the_map_brings_a_tile_left_in_the_air_down_onto_the_sand() {
        // a slot 120 mm over the sand (as a pile laid out by another rule
        // might leave one) and one lying on the sand
        let mut m = Map::flat(2000);
        m.slots.push((Affine::translate(v3(0, mm1(120), 0)), false));
        m.slots.push((Affine::translate(v3(mm1(600), 0, 0)), true));
        let t = crate::tuning::parse(crate::tuning::tests::SHIPPED).unwrap();
        let r = crate::sim::Rules::new(&t);
        m.settle(&r.physics, r.tick_ms as i32);
        assert!(m.settled);
        let (a, _) = m.slots[0];
        assert!(a.t.y.abs() <= mm1(1), "on the sand: {}", a.t.y);
        assert_eq!((a.t.x, a.t.z), (0, 0), "straight down");
        assert_eq!(
            m.slots[1],
            (Affine::translate(v3(mm1(600), 0, 0)), true),
            "the other stays"
        );
    }
}
