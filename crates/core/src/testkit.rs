//! Small models, a font of bars and the shipped look, for the scene and
//! Gallery tests. Their numbers are chosen so expected values can be
//! worked out by hand.

use serde_json::{json, Value};

use crate::glyph::{self, Font, NEEDED};
use crate::look::Look;
use crate::model::{self, Kind, Model};

/// A tile: one 200 mm square on top, 45 mm up, with its two labels.
pub fn tile() -> Value {
    json!({
        "name": "Tile",
        "kind": "tile",
        "unit": "mm",
        "colours": { "wood": [220, 180, 120] },
        "parts": [{ "name": "slab", "vertices": [[-100, 0, -100], [100, 0, -100], [100, 45, 100], [-100, 45, 100]] }],
        "faces": [{ "colour": "wood", "v": [0, 1, 2, 3] }],
        "anchors": {
            "letter": { "part": "slab", "at": [0, 45, 0], "right": [1, 0, 0], "up": [0, 0, -1], "size": [100, 100] },
            "value": { "part": "slab", "at": [60, 45, 70], "right": [1, 0, 0], "up": [0, 0, -1], "size": [60, 40], "align": "right" }
        }
    })
}

/// A crab of two parts: a body (one triangle) and a left claw whose tip,
/// the anchor `claw_left`, is 300 mm forward; grab_left swings the claw
/// 90° toward +x by 200 ms. Its shell takes the player's colour.
pub fn crab() -> Value {
    let mut moves = serde_json::Map::new();
    for anim in Kind::Crab.needs() {
        moves.insert(anim.key().into(), json!({ "ms": 200, "keys": [{ "at": 0 }] }));
    }
    moves.insert(
        "grab_left".into(),
        json!({ "ms": 200, "ease": "linear", "keys": [
            { "at": 0 },
            { "at": 200, "parts": { "claw": { "turn": [0, 90, 0] } } }
        ] }),
    );
    json!({
        "name": "Crab",
        "kind": "crab",
        "unit": "mm",
        "colours": { "shell": { "slot": "player" }, "claw": [200, 60, 40] },
        "parts": [
            { "name": "body", "vertices": [[-200, 0, 0], [200, 0, 0], [0, 100, 0]] },
            { "name": "claw", "parent": "body", "pivot": [0, 50, 0], "vertices": [[0, 50, 0], [0, 50, 300], [0, 80, 300]] }
        ],
        "faces": [{ "colour": "shell", "v": [0, 1, 2] }, { "colour": "claw", "v": [3, 4, 5] }],
        "anchors": {
            "claw_left": { "part": "claw", "at": [0, 50, 300] },
            "claw_right": { "part": "body", "at": [0, 50, 300] }
        },
        "animations": moves
    })
}

pub fn models() -> Vec<Model> {
    model::parse_all(&json!({ "tile": tile(), "crab": crab() }).to_string()).unwrap()
}

/// Every needed character one 20 by 100 bar.
pub fn font_json() -> Value {
    let mut glyphs = serde_json::Map::new();
    for c in NEEDED.chars() {
        glyphs.insert(
            c.to_string(),
            json!({ "width": 20, "polys": [[[0, 0], [20, 0], [20, 100], [0, 100]]] }),
        );
    }
    json!({ "height": 100, "gap": 10, "lift_um": 1000, "glyphs": glyphs })
}

pub fn font() -> Font {
    glyph::parse(&font_json().to_string()).unwrap()
}

pub fn look() -> Look {
    Look::parse(&crate::look::tests::shipped()).unwrap()
}
