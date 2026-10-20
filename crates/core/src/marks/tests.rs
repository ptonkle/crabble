use super::*;
use crate::gfx::render::VERTEX_VALUES;
use crate::testkit;
use serde_json::json;

fn marks(v: serde_json::Value) -> Result<Marks, String> {
    read(&serde_json::from_value(v).unwrap())
}

fn word(from: (i32, i32), to: (i32, i32), owner: usize, real: bool) -> Word {
    Word { from, to, owner, real }
}

/// The example board: CRABS (yellow), SORB (blue), REEF (purple), all
/// real, and FUJ (red), not real.
fn example() -> Vec<Word> {
    vec![
        word((7, 5), (7, 9), 1, true),
        word((4, 8), (7, 8), 2, true),
        word((7, 6), (10, 6), 3, true),
        word((10, 6), (10, 8), 0, false),
    ]
}

#[test]
fn a_square_sits_on_the_board_frame_with_the_centre_at_the_origin() {
    assert_eq!(square_mm((7, 7)), (0, 0));
    // column 9 is two squares toward +x, row 4 three toward -z
    assert_eq!(square_mm((4, 9)), (420, -630));
    assert_eq!(square_mm((0, 14)), (1470, -1470));
}

#[test]
fn a_word_runs_its_squares_first_to_last() {
    assert_eq!(word((7, 5), (7, 9), 0, true).cells(), vec![(7, 5), (7, 6), (7, 7), (7, 8), (7, 9)]);
    assert_eq!(word((4, 8), (6, 8), 0, true).cells(), vec![(4, 8), (5, 8), (6, 8)]);
}

#[test]
fn marks_are_read_with_their_owners_and_checked() {
    let m = marks(json!({
        "words": [{ "from": [7, 5], "to": [7, 9], "owner": "yellow", "real": true }],
        "ghost": [11, 6]
    }))
    .unwrap();
    assert_eq!(m.lie, Lie::Tiles);
    assert_eq!(m.words, vec![word((7, 5), (7, 9), 1, true)]);
    assert_eq!(m.ghost, Some((11, 6)));
    let one = |w: serde_json::Value| marks(json!({ "words": [w] }));
    assert_eq!(
        one(json!({ "from": [7, 5], "to": [8, 6], "owner": "red" })),
        Err("word 0: it must run along a row or a column, from its first square to its last.".into())
    );
    assert_eq!(
        one(json!({ "from": [7, 9], "to": [7, 5], "owner": "red" })),
        Err("word 0: it must run along a row or a column, from its first square to its last.".into())
    );
    assert_eq!(
        one(json!({ "from": [7, 13], "to": [7, 15], "owner": "red" })),
        Err("word 0: a square is off the board (rows and columns are 0 to 14).".into())
    );
    assert_eq!(
        one(json!({ "from": [7, 5], "to": [7, 6], "owner": "green" })),
        Err("word 0: there is no colour \"green\".".into())
    );
    assert_eq!(
        marks(json!({ "on": "sand", "ghost": [1, 1] })),
        Err("the marks lie on \"tiles\" or \"board\", not \"sand\".".into())
    );
    assert_eq!(marks(json!({})), Err("the marks have no words and no ghost.".into()));
}

#[test]
fn a_tile_in_words_of_several_owners_is_shared_in_seat_order() {
    // R in CRABS and REEF, B in CRABS and SORB, F in REEF and FUJ
    assert_eq!(
        shared(&example()),
        vec![((7, 6), vec![1, 3]), ((7, 8), vec![1, 2]), ((10, 6), vec![0, 3])]
    );
    // two words of one owner through a tile share nothing
    let same = [word((7, 5), (7, 7), 2, true), word((5, 6), (8, 6), 2, true)];
    assert_eq!(shared(&same), vec![]);
}

#[test]
fn a_letter_in_any_real_word_takes_the_real_ink() {
    let real = real_cells(&example());
    // F is in REEF; U and J only in FUJ
    assert!(real.contains(&(10, 6)));
    assert!(!real.contains(&(10, 7)) && !real.contains(&(10, 8)));
    // 5 + 4 + 4 squares, less R and B counted twice
    assert_eq!(real.len(), 11);
}

fn band_of(half: i32) -> Band {
    Band {
        x: 0,
        z: 0,
        half: [half, half],
        radius: 20,
        width: 10,
        y: 50,
        lip: 0,
    }
}

fn tris(out: &[i32]) -> usize {
    out.len() / (3 * VERTEX_VALUES)
}

#[test]
fn a_band_is_measured_round_its_middle_line() {
    // four straight sides of 2 × (100 - 20) and four quarter arcs of
    // radius 15: 640 + 2π × 15 = 640 + 94
    assert_eq!(band_of(100).length(), 734);
}

#[test]
fn a_whole_band_is_one_quad_per_side_and_six_per_corner() {
    let paint = Paint::lit([ONE, 0, 0]);
    let mut out = Tris::default();
    band_of(100).draw(50, &[Some(paint)], &mut out);
    // (4 + 4 × 6) quads of two triangles
    assert_eq!(tris(&out.solid), 56);
    // its outer edge reaches 100 mm out and its inner 90, at y = 50
    let xs: Vec<i32> = out.solid.chunks(VERTEX_VALUES).map(|v| v[0]).collect();
    // (to a unit or two of the fixed-point sine)
    let near = |a: i32, b: i32| (a - b).abs() <= 2;
    assert!(near(*xs.iter().max().unwrap(), mm1(100)));
    assert!(xs.iter().any(|x| near(*x, mm1(90))));
    assert!(out.solid.chunks(VERTEX_VALUES).all(|v| v[1] == mm1(50)));
    // a lip adds a wall under each top quad
    let mut out = Tris::default();
    Band { lip: 12, ..band_of(100) }.draw(50, &[Some(paint)], &mut out);
    assert_eq!(tris(&out.solid), 112);
    assert!(out.solid.chunks(VERTEX_VALUES).any(|v| v[1] == mm1(38)));
}

#[test]
fn a_dashed_band_takes_its_colours_in_turn_and_leaves_its_gaps() {
    let red = Paint::lit([ONE, 0, 0]);
    let blue = Paint::lit([0, 0, ONE]);
    let colour_of = |v: &[i32]| [v[6], v[7], v[8]];
    let mut out = Tris::default();
    // 734 mm in dashes of about 50, a whole number of pairs: 14 dashes
    band_of(100).draw(50, &[Some(red), Some(blue)], &mut out);
    let quads: Vec<[i32; 3]> = out.solid.chunks(6 * VERTEX_VALUES).map(colour_of).collect();
    let reds = quads.iter().filter(|c| **c == red.rgb).count();
    let blues = quads.iter().filter(|c| **c == blue.rgb).count();
    assert_eq!(reds + blues, quads.len());
    assert!(reds > 0 && blues > 0);
    // the band starts red, and colours alternate along it
    assert_eq!(quads[0], red.rgb);
    let changes = quads.windows(2).filter(|w| w[0] != w[1]).count();
    assert_eq!(changes, 13);
    // gaps: every other dash is left out
    let mut gapped = Tris::default();
    band_of(100).draw(50, &[Some(red), None], &mut gapped);
    assert_eq!(tris(&gapped.solid), 2 * reds);
}

#[test]
fn the_marks_glow_in_the_owners_colours_round_whole_words() {
    let look = testkit::look();
    let m = Marks {
        lie: Lie::Tiles,
        words: vec![word((7, 5), (7, 9), 1, true)],
        ghost: None,
    };
    let mut out = Tris::default();
    draw(&m, &look, &mut out);
    let yellow = render::rgb(look.players["yellow"]);
    let verts: Vec<&[i32]> = out.solid.chunks(VERTEX_VALUES).collect();
    assert!(verts.iter().all(|v| [v[6], v[7], v[8]] == yellow && v[9] == ONE));
    // one band round the whole run: from square 5's outer edge to square
    // 9's, (7 - 5) × 210 + the inner edge and width out from the middle
    let out_mm = 2 * 210 + look.marks.tiles.inner_mm + look.marks.tiles.width_mm;
    let xs: Vec<i32> = verts.iter().map(|v| v[0]).collect();
    let near = |a: i32, b: i32| (a - b).abs() <= 2;
    assert!(near(*xs.iter().max().unwrap(), mm1(out_mm)));
    assert!(near(*xs.iter().min().unwrap(), mm1(-out_mm)));
}

#[test]
fn shared_tiles_and_the_ghost_are_drawn_over_the_words() {
    let look = testkit::look();
    let step = look.marks.step_mm;
    let m = Marks {
        lie: Lie::Tiles,
        words: example(),
        ghost: Some((11, 6)),
    };
    let mut out = Tris::default();
    draw(&m, &look, &mut out);
    let verts: Vec<&[i32]> = out.solid.chunks(VERTEX_VALUES).collect();
    // the shared tiles' borders sit above all four words' bands
    let top = mm1(look.marks.tiles.y_mm + 4 * step);
    let purple = render::rgb(look.players["purple"]);
    assert!(verts.iter().any(|v| v[1] == top && [v[6], v[7], v[8]] == purple));
    // the ghost's ring is aqua, round its foot on the board, above the rest
    let aqua = render::rgb(look.marks.ghost);
    let ring: Vec<&&[i32]> = verts.iter().filter(|v| [v[6], v[7], v[8]] == aqua).collect();
    assert!(!ring.is_empty());
    assert!(ring.iter().all(|v| v[1] == mm1(look.marks.board.y_mm + 8 * step)));
}
