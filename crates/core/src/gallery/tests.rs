use super::*;
use crate::gfx::render::VERTEX_VALUES;
use crate::testkit;
use serde_json::{json, Value};

const TEXT: &str = include_str!("../../../../data/gallery.json");

fn scenes() -> Value {
    json!({
        "held": {
            "name": "Held",
            "camera": { "target": [0, 0, 0], "yaw": 90, "pitch": 30, "distance": 2000 },
            "things": [
                { "model": "crab", "anim": "grab_left" },
                { "model": "tile", "on": { "thing": 0, "anchor": "claw_left" }, "letter": "E" }
            ]
        }
    })
}

fn loaded() -> Gallery {
    let mut g = Gallery::new();
    let models = json!({ "tile": testkit::tile(), "crab": testkit::crab() });
    g.load(
        &models.to_string(),
        &testkit::font_json().to_string(),
        &scenes().to_string(),
        &crate::look::tests::shipped(),
        TEXT,
    )
    .unwrap();
    g.resize(1600, 900, 100);
    g
}

fn opened(query: &str) -> Gallery {
    let mut g = loaded();
    g.act("gallery.query", query).unwrap();
    g
}

fn items(g: &Gallery) -> Vec<String> {
    g.ui()
        .items
        .iter()
        .map(|i| match i {
            Item::Select { id, .. } | Item::Toggle { id, .. } | Item::Button { id, .. } => id.clone(),
        })
        .collect()
}

/// The camera's eye, from the frame's view values.
fn eye(f: &Frame) -> V3 {
    let (at, _) = render::uniform_at("uEye");
    v3(f.view[at], f.view[at + 1], f.view[at + 2])
}

#[test]
fn the_gallery_opens_from_the_title_and_closes_with_back() {
    let mut g = loaded();
    assert!(!g.ui().open);
    assert_eq!(
        g.frame().unwrap(),
        Frame::default(),
        "nothing is drawn while closed"
    );
    g.act("gallery.open", "").unwrap();
    assert!(g.ui().open);
    assert!(!g.frame().unwrap().tris.is_empty());
    g.act("gallery.close", "").unwrap();
    assert!(!g.ui().open);
    // the models open in Gallery order: the crab first
    assert_eq!(g.ui().heading, "Crab");
}

#[test]
fn a_refused_control_changes_nothing() {
    let mut g = opened("model=tile");
    let before = g.choice.clone();
    assert_eq!(
        g.act("gallery.letter", "a"),
        Err("There is no letter \"a\".".into())
    );
    assert_eq!(
        g.act("gallery.anim", "jump"),
        Err("This model has no move \"jump\".".into())
    );
    assert_eq!(
        g.act("gallery.view", "under"),
        Err("There is no view \"under\".".into())
    );
    assert_eq!(
        g.act("menu.play", ""),
        Err("Unknown action \"menu.play\".".into())
    );
    assert_eq!(
        g.act("gallery.dance", ""),
        Err("Unknown action \"gallery.dance\".".into())
    );
    assert_eq!(
        g.act("gallery.query", "model=tile&ms=soon"),
        Err("The Gallery's \"ms\" takes a whole number, not \"soon\".".into())
    );
    assert_eq!(g.choice, before);
}

#[test]
fn a_file_or_word_missing_is_refused_by_name() {
    let mut g = Gallery::new();
    let mut text: Value = serde_json::from_str(TEXT).unwrap();
    text["strings"].as_object_mut().unwrap().remove("anim.hold");
    let models = json!({ "tile": testkit::tile() }).to_string();
    let e = g.load(
        &models,
        &testkit::font_json().to_string(),
        "{}",
        &crate::look::tests::shipped(),
        &text.to_string(),
    );
    assert_eq!(e, Err("The Gallery's words have no \"anim.hold\".".into()));
    // a scene naming a model that is not there
    let e = g.load(
        &models,
        &testkit::font_json().to_string(),
        &scenes().to_string(),
        &crate::look::tests::shipped(),
        TEXT,
    );
    assert_eq!(e, Err("held: thing 0 (crab): there is no such model.".into()));
    assert!(g.models.is_empty(), "nothing was taken");
}

#[test]
fn the_address_opens_onto_a_tile_with_its_letter_from_above_still() {
    let g = opened("?fast&model=tile&letter=Q&ink=real&view=above&ms=0&bare");
    let ui = g.ui();
    assert!(ui.open && ui.bare);
    assert_eq!(ui.heading, "Tile");
    assert_eq!(ui.sub, "200 × 200 × 45 mm");
    assert_eq!(g.choice.letter, 'Q');
    assert_eq!(g.choice.frozen, Some(0));
    let f = g.frame().unwrap();
    // the floor (48 triangles), the tile (2), "Q" (2), "10" (4)
    assert_eq!(f.tris.len(), (48 + 2 + 2 + 4) * 3 * VERTEX_VALUES);
    // the letter in the real-word ink
    let real = testkit::look().ink(1).rgb;
    let letter = &f.tris[2 * 3 * VERTEX_VALUES..];
    assert_eq!(&letter[6..9], &real);
    // from above: the eye is over the tile
    let e = eye(&f);
    assert!(e.y > 10 * e.z.abs(), "{e:?}");
}

#[test]
fn a_stopped_clock_draws_the_same_frame_whatever_the_time() {
    let mut g = opened("model=crab&anim=grab_left&ms=100");
    g.advance(5_000_000);
    let a = g.frame().unwrap();
    g.advance(9_000_000);
    assert_eq!(g.frame().unwrap(), a);
    assert!(!matches!(
        g.ui().items.iter().find(|i| matches!(i, Item::Toggle { .. })),
        Some(Item::Toggle { pressed: true, .. })
    ));
    // running, the turntable has gone half round in half its turn
    let mut g = opened("model=crab");
    let secs = testkit::look().gallery.turn_seconds as i64;
    g.advance(secs * 1_000_000 / 2);
    assert!(
        (fx::wrap(g.turn_angle() - fx::HALF)).abs() <= 1,
        "{}",
        g.turn_angle()
    );
    // and stops where it is when turned off
    g.act("gallery.turn", "off").unwrap();
    g.advance(secs * 1_000_000);
    assert!((fx::wrap(g.turn_angle() - fx::HALF)).abs() <= 1);
}

#[test]
fn all_four_colours_stand_in_a_row() {
    let g = opened("model=crab&colour=all&ms=0");
    let f = g.frame().unwrap();
    // four crabs of two triangles, and the floor
    assert_eq!(f.tris.len(), (4 * 2 + 48) * 3 * VERTEX_VALUES);
    let look = testkit::look();
    let shells: Vec<[i32; 3]> = (0..4)
        .map(|k| {
            let at = k * 2 * 3 * VERTEX_VALUES + 6;
            [f.tris[at], f.tris[at + 1], f.tris[at + 2]]
        })
        .collect();
    assert_eq!(shells, (0..4).map(|k| look.player(k)).collect::<Vec<_>>());
    // a tile has no slot, so "all four" draws it once
    let g = opened("model=tile&colour=all&ms=0");
    assert_eq!(
        g.frame().unwrap().tris.len(),
        (2 + 2 + 2 + 48) * 3 * VERTEX_VALUES
    );
}

#[test]
fn zooming_in_brings_the_camera_nearer_within_its_bounds() {
    let mut g = opened("model=tile&view=front&ms=0");
    let target = g.models[g.models.iter().position(|m| m.id == "tile").unwrap()]
        .ball()
        .0;
    let far = eye(&g.frame().unwrap()).dist(target);
    g.act("gallery.query", "zoom=200").unwrap();
    let near = eye(&g.frame().unwrap()).dist(target);
    assert!((near * 2 - far).abs() < 64, "{near} {far}");
    // each step is the look's 20%
    g.act("gallery.zoom", "in").unwrap();
    assert_eq!(g.choice.zoom, 240);
    let [lo, hi] = testkit::look().gallery.zoom_pct;
    for _ in 0..20 {
        g.act("gallery.zoom", "in").unwrap();
    }
    assert_eq!(g.choice.zoom, hi);
    for _ in 0..40 {
        g.act("gallery.zoom", "out").unwrap();
    }
    assert_eq!(g.choice.zoom, lo);
    assert_eq!(
        g.act("gallery.query", "zoom=5000"),
        Err(format!("The Gallery's zoom is outside {lo} to {hi}."))
    );
}

#[test]
fn the_views_stand_where_they_say() {
    let target_of = |g: &Gallery| g.models.iter().find(|m| m.id == "tile").unwrap().ball().0;
    for (view, check) in [
        (
            "front",
            (|e: V3| e.z > 0 && e.z > 5 * e.x.abs()) as fn(V3) -> bool,
        ),
        ("above", |e: V3| e.y > 10 * e.z.abs()),
    ] {
        let g = opened(&format!("model=tile&view={view}&ms=0"));
        let e = eye(&g.frame().unwrap()).sub(target_of(&g));
        assert!(check(e), "{view}: {e:?}");
    }
    // the side view turns the model, not the camera: the tile's first
    // corner (-100, 0, -100) turned a quarter round to face the camera
    // with its left (+x) is at (100, 0, -100)
    let g = opened("model=tile&view=side&ms=0");
    let f = g.frame().unwrap();
    let first = v3(f.tris[0], f.tris[1], f.tris[2]);
    assert!(
        first.sub(model::mm([100, 0, -100])).len() < ONE / 500,
        "{first:?}"
    );
    // yaw and pitch from the address stand anywhere
    let g = opened("model=tile&yaw=180&pitch=60&ms=0");
    assert_eq!((g.choice.yaw, g.choice.pitch), (Some(deg(180)), Some(deg(60))));
    assert_eq!(
        loaded().act("gallery.query", "pitch=90"),
        Err("The Gallery's \"pitch\" is outside -89 to 89.".into())
    );
}

#[test]
fn a_tile_turned_down_or_see_through_is_shown_so() {
    let g = opened("model=tile&face=down&alpha=40&ms=0");
    let f = g.frame().unwrap();
    // the floor stays solid; the tile and its letter and value go clear
    assert_eq!(f.tris.len(), 48 * 3 * VERTEX_VALUES);
    assert_eq!(f.clear.len(), (2 + 2 + 2) * 3 * VERTEX_VALUES);
    assert!(g.choice.face_down);
}

#[test]
fn the_controls_suit_the_model() {
    let tile = items(&opened("model=tile"));
    assert!(tile.contains(&"gallery.letter".to_string()) && tile.contains(&"gallery.face".to_string()));
    assert!(
        !tile.contains(&"gallery.colour".to_string()),
        "a tile has no player colour"
    );
    let crab = items(&opened("model=crab"));
    assert!(crab.contains(&"gallery.colour".to_string()) && !crab.contains(&"gallery.letter".to_string()));
    // the crab's moves: still and its nine
    let ui = opened("model=crab").ui();
    let moves = ui
        .items
        .iter()
        .find_map(|i| match i {
            Item::Select { id, options, .. } if id == "gallery.anim" => Some(options.len()),
            _ => None,
        })
        .unwrap();
    assert_eq!(moves, 10);
    assert_eq!(crab.last().unwrap(), "gallery.close");
}

#[test]
fn a_scene_opens_with_its_camera_and_draws_its_things() {
    let g = opened("scene=held&ms=0");
    let ui = g.ui();
    assert_eq!(ui.heading, "Held");
    assert!(!items(&g).contains(&"gallery.anim".to_string()));
    let f = g.frame().unwrap();
    // the crab (2 triangles), the tile (2) and its "E" (2) and "1" (2); no floor
    assert_eq!(f.tris.len(), 8 * 3 * VERTEX_VALUES);
    // the camera: 2 m out toward +x (yaw 90°), 30° up
    let e = eye(&f);
    assert!(e.sub(v3(ONE * 1732 / 1000, ONE, 0)).len() < ONE / 200, "{e:?}");
    // the scene's time runs on from each thing's own
    let a = opened("scene=held&ms=0").frame().unwrap();
    let b = opened("scene=held&ms=200").frame().unwrap();
    assert_ne!(a.tris, b.tris);
    assert_eq!(
        loaded().act("gallery.query", "scene=nowhere"),
        Err("There is no scene \"nowhere\".".into())
    );
    // choosing no scene goes back to the first model
    let mut g = opened("scene=held");
    g.act("gallery.scene", "").unwrap();
    assert_eq!(g.choice.show, Show::Model(0));
}

#[test]
fn the_door_wraps_answers_in_ok_or_error() {
    let mut g = loaded();
    let v: Value = serde_json::from_str(&g.act_json("gallery.open", "")).unwrap();
    assert_eq!(v["ok"]["open"], true);
    assert_eq!(v["ok"]["items"][0]["kind"], "select");
    let v: Value = serde_json::from_str(&g.act_json("gallery.zoom", "sideways")).unwrap();
    assert_eq!(v["error"], "The zoom is \"in\" or \"out\", not \"sideways\".");
    let mut empty = Gallery::new();
    assert_eq!(
        empty.act("gallery.query", "gallery"),
        Err("The Gallery has no models.".into())
    );
}
