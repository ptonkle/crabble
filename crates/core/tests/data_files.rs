//! The real data files: every model, scene and glyph file the build ships
//! is one the core accepts, and together they open the Gallery; the menus'
//! files are accepted whole, and What's New is this version's.

use std::fs;
use std::path::{Path, PathBuf};

use tool_core::{gallery, glyph, model, scene};

fn data(name: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Each `<id>.json` under `data/<dir>`, as (id, text), in id order.
fn files(dir: &str) -> Vec<(String, String)> {
    let at = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data").join(dir);
    let mut paths: Vec<PathBuf> = fs::read_dir(&at)
        .unwrap_or_else(|e| panic!("data/{dir}: {e}"))
        .map(|e| e.expect("entry").path())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            assert_eq!(
                path.extension().and_then(|x| x.to_str()),
                Some("json"),
                "{}",
                path.display()
            );
            let id = path.file_stem().unwrap().to_str().unwrap().to_string();
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{id}: a file's name is lowercase words joined by hyphens"
            );
            (id, fs::read_to_string(&path).unwrap())
        })
        .collect()
}

/// The files bundled as the build bundles them: one `{id: file}` object.
fn bundle(dir: &str) -> String {
    let parts: Vec<String> = files(dir)
        .into_iter()
        .map(|(id, text)| format!("\"{id}\":{text}"))
        .collect();
    format!("{{{}}}", parts.join(","))
}

/// Every model file is one the core accepts, under the name the Gallery and
/// the game will know it by.
#[test]
fn every_model_file_is_accepted() {
    let all = files("models");
    assert!(!all.is_empty(), "data/models holds no model");
    for (id, text) in all {
        if let Err(e) = model::parse(&id, &text) {
            panic!("{e}");
        }
    }
}

/// The starter tile is the agreed size (SECOND-ORDER R1.8) and has its
/// letter and value labels on its top face.
#[test]
fn the_tile_is_two_hundred_by_two_hundred_by_forty_five() {
    let t = model::parse("tile", &data("models/tile.json")).unwrap();
    assert_eq!((t.width_mm(), t.length_mm(), t.height_mm()), (200, 200, 45));
    assert!(t.has_label("letter") && t.has_label("value"));
}

#[test]
fn the_glyphs_draw_every_letter_and_digit() {
    let font = glyph::parse(&data("glyphs.json")).unwrap_or_else(|e| panic!("{e}"));
    for c in glyph::NEEDED.chars() {
        assert!(font.glyphs.contains_key(&c), "{c}");
    }
}

/// Every scene file reads, and names only models, animations, anchors and
/// letters there are.
#[test]
fn every_scene_file_is_accepted_against_the_models() {
    let models = model::parse_all(&bundle("models")).unwrap();
    let font = glyph::parse(&data("glyphs.json")).unwrap();
    let all = files("scenes");
    assert!(!all.is_empty(), "data/scenes holds no scene");
    for (id, text) in all {
        let s = scene::parse(&id, &text).unwrap_or_else(|e| panic!("{e}"));
        if let Err(e) = s.check(&models, &font) {
            panic!("{e}");
        }
    }
}

/// The shipped files open the Gallery, and each scene draws.
#[test]
fn the_shipped_files_open_the_gallery_and_every_scene_draws() {
    let mut g = gallery::Gallery::new();
    g.load(
        &bundle("models"),
        &data("glyphs.json"),
        &bundle("scenes"),
        &data("look.json"),
        &data("gallery.json"),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    g.resize(1920, 1080, 100);
    for (id, _) in files("scenes") {
        g.act("gallery.query", &format!("scene={id}&ms=0")).unwrap();
        let f = g.frame().unwrap_or_else(|e| panic!("{id}: {e}"));
        assert!(!f.tris.is_empty(), "{id} draws nothing");
    }
}

#[test]
fn the_newest_patch_notes_are_this_version() {
    let notes = tool_core::patch_notes::parse(&data("patch-notes.json")).unwrap();
    assert_eq!(
        notes[0].version,
        tool_core::app::VERSION,
        "scripts/bump.sh moves VERSION and the notes together"
    );
}

#[test]
fn the_strings_tuning_look_and_sounds_files_are_accepted_whole() {
    let app = tool_core::app::App::new(&data("strings.json"), &data("patch-notes.json"));
    assert!(app.is_ok(), "{:?}", app.err());
    let mut app = app.unwrap();
    app.act("data.tuning", &data("tuning.json")).unwrap();
    // every sound the game names is in the sounds file
    app.act("data.sounds", &data("sounds.json")).unwrap();
}

/// The game on the shipped map: the crabs stand on its ground, every
/// tile is dealt to a slot, and a frame draws the main view and the
/// overhead view with the world and the tiles in static layers.
#[test]
fn a_game_on_the_shipped_map_stands_its_crabs_on_the_ground_and_draws_two_passes() {
    let mut app = tool_core::app::App::new(&data("strings.json"), &data("patch-notes.json")).unwrap();
    app.act("data.tuning", &data("tuning.json")).unwrap();
    app.act("data.models", &bundle("models")).unwrap();
    app.act("data.glyphs", &data("glyphs.json")).unwrap();
    app.act("data.look", &data("look.json")).unwrap();
    app.act("data.map", &data("scenes/seafloor.json")).unwrap();
    app.act("flag.fast", "on").unwrap();
    app.act("local.ai", "3").unwrap();
    app.act("local.start", "").unwrap();
    app.resize(800, 600, 100);
    let g = &app.play.as_ref().unwrap().game;
    assert_eq!(g.tiles.all.len(), 294, "three sets, no blanks");
    // the corners are flat sand, a little over the floor's 0
    for c in &g.crabs {
        let mm = c.pos.y as i64 * 1000 / tool_core::gfx::fx::ONE as i64;
        assert!((-5..=40).contains(&mm), "a crab stands at {mm} mm");
    }
    app.build_frame();
    let info = &app.view.info;
    assert_eq!(info.passes.len(), 2);
    assert_eq!(
        info.passes[1].viewport[0], info.passes[1].viewport[1],
        "the overhead view sits in its corner"
    );
    // the world, then the tiles in buckets of 30
    assert_eq!(info.layers.len(), 1 + 10);
    assert!(app.layer(0).len() > 1000, "the world has triangles");
    assert!(!app.layer(1).is_empty(), "the first tiles are drawn at rest");
    assert!(
        !app.view.frame[0].solid.is_empty(),
        "the crabs are drawn each frame"
    );
}
