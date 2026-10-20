//! The shipped data files, read as the page hands them to the core.

use std::fs;
use std::path::Path;

fn read(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {name}: {e}"))
}

#[test]
fn the_newest_patch_notes_are_this_version() {
    let notes = tool_core::patch_notes::parse(&read("data/patch-notes.json")).unwrap();
    assert_eq!(
        notes[0].version,
        tool_core::app::VERSION,
        "scripts/bump.sh moves VERSION and the notes together"
    );
}

#[test]
fn the_strings_tuning_and_look_files_are_accepted_whole() {
    let app = tool_core::app::App::new(&read("data/strings.json"), &read("data/patch-notes.json"));
    assert!(app.is_ok(), "{:?}", app.err());
    let mut app = app.unwrap();
    app.act("data.tuning", &read("data/tuning.json")).unwrap();
    app.act("data.standin", &read("data/standin.json")).unwrap();
}
