use super::*;
use crate::gfx::render::VERTEX_VALUES;
use crate::testkit;
use serde_json::json;

fn marks(v: serde_json::Value) -> Result<Marks, String> {
    read(&serde_json::from_value(v).unwrap())
}

fn word(from: (i32, i32), to: (i32, i32), owner: usize, real: bool) -> Word {
    Word {
        from,
        to,
        owner,
        real,
    }
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
    assert_eq!(
        word((7, 5), (7, 9), 0, true).cells(),
        vec![(7, 5), (7, 6), (7, 7), (7, 8), (7, 9)]
    );
    assert_eq!(
        word((4, 8), (6, 8), 0, true).cells(),
        vec![(4, 8), (5, 8), (6, 8)]
    );
}

#[test]
fn marks_are_read_with_their_owners_and_checked() {
    let m = marks(json!({
        "words": [{ "from": [7, 5], "to": [7, 9], "owner": "yellow", "real": true }],
        "ghost": [11, 6]
    }))
    .unwrap();
    assert_eq!(m.me, None);
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
        marks(json!({ "me": "blue", "ghost": [1, 1] })).unwrap().me,
        Some(2)
    );
    assert_eq!(
        marks(json!({ "me": "sand", "ghost": [1, 1] })),
        Err("there is no colour \"sand\" for \"me\".".into())
    );
    assert_eq!(
        marks(json!({})),
        Err("the marks have no words and no ghost.".into())
    );
}

#[test]
fn a_letter_takes_the_real_ink_only_when_every_word_through_it_is_real() {
    let real = real_cells(&example());
    // F is in REEF and in FUJ, which is not real; U and J only in FUJ
    assert!(!real.contains(&(10, 6)));
    assert!(!real.contains(&(10, 7)) && !real.contains(&(10, 8)));
    // R is in CRABS and REEF, both real
    assert!(real.contains(&(7, 6)));
    // 5 + 4 + 4 squares, less R and B counted twice, less F
    assert_eq!(real.len(), 10);
}

fn sides_of(edges: &[Edge], cell: (i32, i32)) -> Vec<usize> {
    edges.iter().filter(|e| e.cell == cell).map(|e| e.side).collect()
}

#[test]
fn a_word_is_outlined_all_the_way_round_and_never_inside() {
    // three across: the top and bottom of each, and each end
    let e = edges(&[word((7, 5), (7, 7), 1, true)], None);
    assert_eq!(e.len(), 8);
    assert_eq!(sides_of(&e, (7, 5)), vec![0, 2, 3]);
    assert_eq!(sides_of(&e, (7, 6)), vec![0, 2]);
    assert_eq!(sides_of(&e, (7, 7)), vec![0, 1, 2]);
    assert!(e.iter().all(|e| e.owner == 1 && e.real));
}

#[test]
fn two_crossing_words_of_one_owner_share_one_outline() {
    // a plus: across (7,5)-(7,7), down (6,6)-(8,6); its outline is the
    // plus's 12 sides, none through the middle square
    let plus = [word((7, 5), (7, 7), 1, true), word((6, 6), (8, 6), 1, true)];
    let e = edges(&plus, Some(1));
    assert_eq!(e.len(), 12);
    assert_eq!(sides_of(&e, (7, 6)), Vec::<usize>::new());
    assert_eq!(sides_of(&e, (6, 6)), vec![0, 1, 3]);
    // the down word another player's: only the across word, whole
    let theirs = [word((7, 5), (7, 7), 1, true), word((6, 6), (8, 6), 2, true)];
    let e = edges(&theirs, Some(1));
    assert_eq!(e.len(), 8);
    assert_eq!(sides_of(&e, (7, 6)), vec![0, 2]);
    assert!(e.iter().all(|e| e.owner == 1));
}

#[test]
fn side_by_side_words_keep_their_own_outlines() {
    // two across words of one owner, one under the other, joined by two
    // down words of another: the sides between them stay
    let words = [
        word((5, 3), (5, 5), 1, true),
        word((6, 3), (6, 4), 1, true),
        word((5, 3), (6, 3), 0, true),
        word((5, 4), (6, 4), 0, true),
    ];
    let e = edges(&words, Some(1));
    assert_eq!(e.len(), 8 + 6);
    assert_eq!(sides_of(&e, (5, 3)), vec![0, 2, 3]);
    assert_eq!(sides_of(&e, (6, 3)), vec![0, 2, 3]);
    // drawing every owner's, the other owner's words do not open them
    let e: Vec<Edge> = edges(&words, None).into_iter().filter(|e| e.owner == 1).collect();
    assert_eq!(e.len(), 8 + 6);
    assert_eq!(sides_of(&e, (5, 3)), vec![0, 2, 3]);
}

#[test]
fn a_side_on_a_real_and_a_fake_outline_is_drawn_once_dashed() {
    // CAT across, real; a fake word down from its T
    let words = [word((7, 5), (7, 7), 1, true), word((7, 7), (9, 7), 1, false)];
    let e = edges(&words, Some(1));
    // 7 sides each, the T's top and right counted once
    assert_eq!(e.len(), 12);
    let at = |cell, side| e.iter().find(|e| e.cell == cell && e.side == side).copied();
    assert_eq!(at((7, 7), 0).map(|e| e.real), Some(false));
    assert_eq!(at((7, 7), 1).map(|e| e.real), Some(false));
    assert_eq!(at((7, 5), 0).map(|e| e.real), Some(true));
    // neither runs through the other
    assert_eq!(at((7, 7), 2), None);
    assert_eq!(at((7, 7), 3), None);
}

#[test]
fn a_fake_outline_is_dashed_along_each_side_and_a_real_one_whole() {
    assert_eq!(stretches(true, 118, 3, 55), vec![(-118, 118)]);
    // a share of 70 each, a dash of 38 in its middle
    assert_eq!(
        stretches(false, 118, 3, 55),
        vec![(-89, -51), (-19, 19), (51, 89)]
    );
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
    Band {
        lip: 12,
        ..band_of(100)
    }
    .draw(50, &[Some(paint)], &mut out);
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
fn only_my_words_are_outlined_in_my_colour_at_the_board() {
    let look = testkit::look();
    let o = &look.marks.outline;
    let m = Marks {
        me: Some(1),
        words: example(),
        ghost: None,
    };
    let mut out = Tris::default();
    draw(&m, &look, &mut out);
    let yellow = render::rgb(look.players["yellow"]);
    let verts: Vec<&[i32]> = out.solid.chunks(VERTEX_VALUES).collect();
    assert!(!verts.is_empty());
    assert!(verts.iter().all(|v| [v[6], v[7], v[8]] == yellow && v[9] == ONE));
    // CRABS alone: 12 sides, each a foot, a wall and a rim of 2 triangles
    assert_eq!(verts.len(), 12 * 3 * 2 * 3);
    // from the board at the tiles' feet up to their tops, never above
    assert!(verts
        .iter()
        .all(|v| v[1] == mm1(o.foot_y_mm) || v[1] == mm1(o.top_y_mm)));
    // round the whole run: S's foot reaches (9 - 7) × 210 + foot_out
    let xs: Vec<i32> = verts.iter().map(|v| v[0]).collect();
    assert_eq!(*xs.iter().max().unwrap(), mm1(2 * 210 + o.foot_out_mm));
    assert_eq!(*xs.iter().min().unwrap(), mm1(-2 * 210 - o.foot_out_mm));
    // the foot reaches round S's outer corner
    let corner = (mm1(2 * 210 + o.foot_out_mm), mm1(o.foot_out_mm));
    assert!(verts.iter().any(|v| (v[0], v[2]) == corner));
}

#[test]
fn a_fake_word_is_drawn_in_dashes_and_the_ghost_ring_over_the_board() {
    let look = testkit::look();
    let o = &look.marks.outline;
    let count = |real: bool, ghost: Option<(i32, i32)>| {
        let m = Marks {
            me: None,
            words: vec![word((10, 6), (10, 8), 0, real)],
            ghost,
        };
        let mut out = Tris::default();
        draw(&m, &look, &mut out);
        out
    };
    // 8 sides, each in `dashes` pieces of a foot, a wall and a rim
    assert_eq!(tris(&count(true, None).solid), 8 * 3 * 2);
    assert_eq!(tris(&count(false, None).solid), 8 * 3 * 2 * o.dashes as usize);
    // the ghost's ring is aqua, round its foot on the board
    let out = count(true, Some((11, 6)));
    let aqua = render::rgb(look.marks.ghost);
    let ring: Vec<&[i32]> = out
        .solid
        .chunks(VERTEX_VALUES)
        .filter(|v| [v[6], v[7], v[8]] == aqua)
        .collect();
    assert!(!ring.is_empty());
    assert!(ring
        .iter()
        .all(|v| v[1] == mm1(look.marks.board.y_mm + look.marks.step_mm)));
}
