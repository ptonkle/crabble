use super::*;
use crate::gfx::fx::int;
use crate::gfx::render::VERTEX_VALUES;
use serde_json::{json, Value};

/// A one-triangle prop: three corners a metre out on x, y and z.
fn tiny() -> Value {
    json!({
        "name": "Tiny",
        "kind": "prop",
        "unit": "mm",
        "colours": { "red": [255, 0, 0] },
        "parts": [{ "name": "a", "vertices": [[1000, 0, 0], [0, 1000, 0], [0, 0, 1000]] }],
        "faces": [{ "colour": "red", "v": [0, 1, 2] }]
    })
}

/// Two parts: `arm` (one triangle at x 1 m) hangs from `base` (one at the
/// origin), its pivot at x 1 m.
fn jointed() -> Value {
    json!({
        "name": "Jointed",
        "kind": "prop",
        "unit": "mm",
        "colours": { "grey": [128, 128, 128], "shell": { "slot": "player", "shade": 50 } },
        "parts": [
            { "name": "base", "vertices": [[0, 0, 0], [0, 100, 0], [0, 0, 100]] },
            { "name": "arm", "parent": "base", "pivot": [1000, 0, 0],
              "vertices": [[2000, 0, 0], [2000, 100, 0], [2000, 0, 100]] }
        ],
        "faces": [{ "colour": "grey", "v": [0, 1, 2] }, { "colour": "shell", "v": [3, 4, 5] }],
        "anchors": { "claw_left": { "part": "arm", "at": [2000, 0, 0] } },
        "animations": {
            "sway": { "ms": 1000, "keys": [
                { "at": 0 },
                { "at": 500, "parts": { "arm": { "turn": [0, 90, 0] } } }
            ] },
            "idle": { "ms": 100, "keys": [
                { "at": 0, "parts": { "base": { "turn": [0, 90, 0] }, "arm": { "scale": 0 } } }
            ] }
        }
    })
}

/// Every animation a crab needs, each one key at rest.
fn crab_moves() -> Value {
    let mut a = serde_json::Map::new();
    for anim in Kind::Crab.needs() {
        a.insert(anim.key().into(), json!({ "ms": 100, "keys": [{ "at": 0 }] }));
    }
    Value::Object(a)
}

/// A tile: one square on top, 45 mm up, with its two labels.
fn tile() -> Value {
    json!({
        "name": "Tile",
        "kind": "tile",
        "unit": "mm",
        "colours": { "wood": [220, 180, 120] },
        "parts": [{ "name": "slab", "vertices": [[-100, 45, -100], [100, 45, -100], [100, 45, 100], [-100, 45, 100]] }],
        "faces": [{ "colour": "wood", "v": [0, 1, 2, 3] }],
        "anchors": {
            "letter": { "part": "slab", "at": [0, 45, 0], "right": [1, 0, 0], "up": [0, 0, -1], "size": [150, 150] },
            "value": { "part": "slab", "at": [60, 45, 70], "right": [1, 0, 0], "up": [0, 0, -1], "size": [60, 40], "align": "right" }
        }
    })
}

fn load(v: &Value) -> Result<Model, String> {
    parse("m", &v.to_string())
}

fn refused(v: Value, msg: &str) {
    assert_eq!(load(&v), Err(format!("m: {msg}")));
}

fn near(a: V3, b: V3) -> bool {
    a.sub(b).len() < ONE / 500
}

fn drawn(m: &Model, poses: &[Pose]) -> Tris {
    let mut out = Tris::default();
    m.draw(
        &m.transforms(poses),
        &Affine::IDENTITY,
        &m.paints([0; 3]),
        &mut out,
    );
    out
}

#[test]
fn a_model_file_becomes_flat_triangles_in_metres() {
    let m = load(&tiny()).unwrap();
    assert_eq!(
        (m.id.as_str(), m.name.as_str(), m.kind),
        ("m", "Tiny", Kind::Prop)
    );
    let out = drawn(&m, &m.poses(None, 0)).solid;
    assert_eq!(out.len(), 3 * VERTEX_VALUES);
    // 1000 mm is one metre
    assert_eq!(&out[0..3], &[ONE, 0, 0]);
    // the face (1,0,0) (0,1,0) (0,0,1) faces out along (1,1,1)/√3: 37837 each
    for n in &out[3..6] {
        assert!((n - 37_837).abs() < 8, "{n}");
    }
    // red, lit, solid
    assert_eq!(&out[6..11], &[ONE, 0, 0, 0, ONE]);
    assert_eq!((m.length_mm(), m.height_mm(), m.width_mm()), (1000, 1000, 1000));
}

#[test]
fn a_four_cornered_face_is_two_triangles() {
    let mut v = tiny();
    v["parts"][0]["vertices"] = json!([[0, 0, 0], [1000, 0, 0], [1000, 0, 1000], [0, 0, 1000]]);
    v["faces"][0]["v"] = json!([0, 1, 2, 3]);
    let m = load(&v).unwrap();
    let out = drawn(&m, &m.poses(None, 0)).solid;
    assert_eq!(out.len(), 6 * VERTEX_VALUES);
    // wound from +x toward +z about the origin: its normal is -y
    assert_eq!(&out[3..6], &[0, -ONE, 0]);
}

#[test]
fn a_file_in_another_unit_or_kind_or_with_no_name_is_refused() {
    let mut v = tiny();
    v["unit"] = json!("cm");
    refused(v, "the unit is \"cm\"; model files use \"mm\".");
    let mut v = tiny();
    v["kind"] = json!("fish");
    refused(v, "there is no kind \"fish\".");
    let mut v = tiny();
    v["name"] = json!(" ");
    refused(v, "the model has no name.");
    let mut v = tiny();
    v["colour"] = json!("x");
    assert!(load(&v).unwrap_err().contains("unknown field `colour`"));
}

#[test]
fn a_bad_colour_is_refused_by_name() {
    let mut v = tiny();
    v["colours"]["red"] = json!([256, 0, 0]);
    refused(v, "the colour \"red\" is outside 0 to 255.");
    let mut v = tiny();
    v["colours"]["red"] = json!({ "slot": "rival" });
    refused(v, "the colour \"red\" names no slot \"rival\".");
    let mut v = tiny();
    v["colours"]["red"] = json!({ "slot": "player", "shade": 201 });
    refused(v, "the colour \"red\" has a shade outside 0 to 200.");
    let mut v = tiny();
    v["colours"]["blue"] = json!([0, 0, 255]);
    refused(v, "the colour \"blue\" is on no face.");
    let mut v = tiny();
    v["colours"]["red"] = json!({ "rgb": [255, 0, 0], "alpha": 0 });
    refused(v, "the colour \"red\" has an alpha outside 1 to 100.");
    let mut v = tiny();
    v["colours"]["red"] = json!({ "slot": "player", "alpha": 101 });
    refused(v, "the colour \"red\" has an alpha outside 1 to 100.");
}

#[test]
fn a_bad_face_is_refused_by_number() {
    let mut v = tiny();
    v["faces"][0]["colour"] = json!("pink");
    refused(v, "face 0 names the colour \"pink\", which is not listed.");
    let mut v = tiny();
    v["faces"][0]["v"] = json!([0, 1]);
    refused(v, "face 0 has fewer than 3 corners.");
    let mut v = tiny();
    v["faces"][0]["v"] = json!([0, 1, 3]);
    refused(v, "face 0 names vertex 3; there are 3.");
    let mut v = tiny();
    v["faces"][0]["v"] = json!([0, 1, 1]);
    refused(v, "face 0 names vertex 1 twice.");
    let mut v = tiny();
    v["parts"][0]["vertices"]
        .as_array_mut()
        .unwrap()
        .push(json!([5, 5, 5]));
    refused(v, "vertex 3 is in no face.");
    let mut v = tiny();
    v["faces"] = json!([]);
    refused(v, "the model has no faces.");
}

#[test]
fn a_bad_part_is_refused_by_name() {
    let mut v = jointed();
    v["parts"][1]["name"] = json!("base");
    refused(v, "there are two parts \"base\".");
    let mut v = jointed();
    v["parts"][1]["parent"] = json!("arm");
    refused(
        v,
        "the part \"arm\" has a parent \"arm\" that is not listed before it.",
    );
    let mut v = jointed();
    v["parts"][1]["pivot"] = json!([30_001, 0, 0]);
    refused(v, "the part \"arm\" reaches past 30000 mm.");
    let mut v = jointed();
    v["anchors"]["claw_left"]["part"] = json!("leg");
    refused(v, "the anchor \"claw_left\" names no part \"leg\".");
}

#[test]
fn a_bad_animation_is_refused_by_name() {
    let mut v = jointed();
    v["animations"]["dance"] = json!({ "ms": 10, "keys": [{ "at": 0 }] });
    refused(v, "there is no animation \"dance\".");
    let mut v = jointed();
    v["animations"]["sway"]["keys"][1]["at"] = json!(0);
    refused(
        v,
        "the animation \"sway\" has a key at 0 out of order or past its end.",
    );
    let mut v = jointed();
    v["animations"]["sway"]["keys"][1]["at"] = json!(1000);
    refused(
        v,
        "the animation \"sway\" has a key at 1000 out of order or past its end.",
    );
    let mut v = jointed();
    v["animations"]["sway"]["keys"][0]["at"] = json!(10);
    refused(v, "the animation \"sway\" must start with a key at 0.");
    let mut v = jointed();
    v["animations"]["sway"]["ms"] = json!(0);
    refused(v, "the animation \"sway\" lasts 0 ms; it must be 1 to 60000.");
    let mut v = jointed();
    v["animations"]["sway"]["keys"][1]["parts"]["leg"] = json!({});
    refused(v, "the animation \"sway\" names no part \"leg\".");
    let mut v = jointed();
    v["animations"]["sway"]["keys"][1]["parts"]["arm"]["scale"] = json!(401);
    refused(v, "the animation \"sway\" poses \"arm\" out of range.");
}

#[test]
fn a_crab_needs_every_move_the_game_plays_and_both_claw_anchors() {
    let mut v = jointed();
    v["kind"] = json!("crab");
    v["anchors"]["claw_right"] = json!({ "part": "arm", "at": [2000, 0, 0] });
    v["animations"] = crab_moves();
    assert!(load(&v).is_ok());
    // each one, taken away, is named
    for anim in Kind::Crab.needs() {
        let mut short = v.clone();
        short["animations"].as_object_mut().unwrap().remove(anim.key());
        refused(
            short,
            &format!("a crab model needs the animation \"{}\".", anim.key()),
        );
    }
    let mut short = v.clone();
    short["anchors"].as_object_mut().unwrap().remove("claw_right");
    refused(short, "a crab model needs the anchor \"claw_right\".");
    // the brief's nine moves, and the one-claw holds (CR-Q9)
    let keys: Vec<&str> = Kind::Crab.needs().iter().map(|a| a.key()).collect();
    assert_eq!(
        keys,
        [
            "idle",
            "walk_side",
            "walk_fwd",
            "jump",
            "grab_left",
            "grab_right",
            "hold",
            "hold_left",
            "hold_right",
            "place_left",
            "place_right"
        ]
    );
}

#[test]
fn a_tile_needs_its_letter_and_value_labels() {
    assert!(load(&tile()).is_ok());
    let mut v = tile();
    v["anchors"].as_object_mut().unwrap().remove("value");
    refused(v, "a tile model needs the anchor \"value\".");
    let mut v = tile();
    v["anchors"]["letter"] = json!({ "part": "slab", "at": [0, 45, 0] });
    refused(v, "a tile model's anchor \"letter\" must be a label.");
}

#[test]
fn a_bad_label_is_refused_by_name() {
    let mut v = tile();
    v["anchors"]["letter"].as_object_mut().unwrap().remove("size");
    refused(
        v,
        "the anchor \"letter\" is a label only with all of right, up and size.",
    );
    let mut v = tile();
    v["anchors"]["letter"]["up"] = json!([1, 0, 1]);
    refused(v, "the label \"letter\" has right and up not at right angles.");
    let mut v = tile();
    v["anchors"]["letter"]["up"] = json!([0, 0, 0]);
    refused(v, "the label \"letter\" has no direction.");
    let mut v = tile();
    v["anchors"]["letter"]["size"] = json!([0, 150]);
    refused(v, "the label \"letter\" has a size outside 1 to 30000 mm.");
    let mut v = tile();
    v["anchors"]["letter"]["align"] = json!("left");
    refused(v, "the label \"letter\" has no align \"left\".");
    let mut v = jointed();
    v["anchors"]["claw_left"]["align"] = json!("right");
    refused(v, "the anchor \"claw_left\" has an align but is no label.");
}

#[test]
fn a_label_lies_on_its_part_facing_out_of_right_and_up() {
    let m = load(&tile()).unwrap();
    let t = m.transforms(&m.poses(None, 0));
    let l = m.label("letter", &t, &Affine::IDENTITY).unwrap();
    assert!(near(l.at, v3(0, ONE * 45 / 1000, 0)));
    // right +x, up -z: out of the face is +y
    assert!(near(l.out, v3(0, ONE, 0)), "{:?}", l.out);
    assert_eq!(l.size, [mm1(150), mm1(150)]);
    assert_eq!(
        m.label("value", &t, &Affine::IDENTITY).unwrap().align,
        Align::Right
    );
    // turned face down (180° about z), the label faces -y and moves under
    let flip = Affine::rot_z(deg(180));
    let l = m.label("letter", &t, &flip).unwrap();
    assert!(near(l.out, v3(0, -ONE, 0)), "{:?}", l.out);
    assert!(near(l.at, v3(0, -ONE * 45 / 1000, 0)));
    // twice the size, twice the label
    let big = Affine::scale(v3(int(2), int(2), int(2)));
    assert_eq!(m.label("letter", &t, &big).unwrap().size[0], mm1(300));
    // a point anchor is no label
    let j = load(&jointed()).unwrap();
    assert!(j
        .label("claw_left", &j.transforms(&j.poses(None, 0)), &Affine::IDENTITY)
        .is_none());
    assert!(m.has_label("letter") && !j.has_label("claw_left"));
}

#[test]
fn a_part_turns_about_its_pivot_easing_between_keys() {
    let m = load(&jointed()).unwrap();
    let tip = |ms: u32| {
        let t = m.transforms(&m.poses(Some(Anim::Sway), ms));
        t[1].apply(m.verts[3].0)
    };
    // at rest the arm's tip is 2 m out on x
    assert!(near(tip(0), v3(int(2), 0, 0)));
    // at 500 ms it has turned 90° about its pivot (x 1 m): +x turns to -z
    assert!(near(tip(500), v3(ONE, 0, -ONE)), "{:?}", tip(500));
    // halfway there it is at 45° (easing is even at the middle)
    let h = ONE * 7071 / 10_000;
    assert!(near(tip(250), v3(ONE + h, 0, -h)), "{:?}", tip(250));
    // a quarter of the way there, eased: 3/16 − 2/64 of 90° = 14.0625°
    // (cos 0.97003, sin 0.24298), not 22.5°
    let q = tip(125);
    assert!(
        near(q, v3(ONE + ONE * 9700 / 10_000, 0, -ONE * 2430 / 10_000)),
        "{q:?}"
    );
    // straight, not eased, when the animation says linear: a quarter of 90°
    let mut v = jointed();
    v["animations"]["sway"]["ease"] = json!("linear");
    let lin = load(&v).unwrap();
    let t = lin.transforms(&lin.poses(Some(Anim::Sway), 125));
    let (c, s) = (ONE * 9239 / 10_000, ONE * 3827 / 10_000);
    assert!(near(t[1].apply(lin.verts[3].0), v3(ONE + c, 0, -s)));
    v["animations"]["sway"]["ease"] = json!("bouncy");
    refused(v, "the animation \"sway\" has no ease \"bouncy\".");
    // it eases back after the last key, and loops
    assert!(near(tip(750), tip(250)));
    assert!(near(tip(1000), tip(0)));
}

#[test]
fn a_move_played_once_holds_its_last_key_and_may_end_on_its_length() {
    let mut v = jointed();
    // a jump: up 300 mm by 400 ms, held there after
    v["animations"]["jump"] = json!({ "ms": 400, "ease": "linear", "keys": [
        { "at": 0 },
        { "at": 400, "parts": { "base": { "move": [0, 300, 0] } } }
    ] });
    let m = load(&v).unwrap();
    let up = |ms: u32| m.transforms(&m.poses(Some(Anim::Jump), ms))[0].apply(V3::ZERO).y;
    assert_eq!(up(0), 0);
    assert!((up(200) - mm1(150)).abs() <= 2, "{}", up(200));
    // not back to the start at its length or after, as a loop would be
    assert!((up(400) - mm1(300)).abs() <= 2);
    assert!((up(5000) - mm1(300)).abs() <= 2);
    // a loop may not put a key at its length
    let mut v = jointed();
    v["animations"]["idle"]["keys"] = json!([{ "at": 0 }, { "at": 100 }]);
    refused(
        v,
        "the animation \"idle\" has a key at 100 out of order or past its end.",
    );
    assert!(Anim::Jump.once() && Anim::PlaceRight.once() && !Anim::Hold.once() && !Anim::WalkSide.once());
}

#[test]
fn a_later_layer_takes_over_only_the_parts_it_moves() {
    let m = load(&jointed()).unwrap();
    // idle turns the base and hides the arm; sway moves only the arm
    let base_turn = |p: &[Pose]| p[0].turn[1];
    let arm_scale = |p: &[Pose]| p[1].scale;
    let both = m.layered(&[(Anim::Idle, 0), (Anim::Sway, 500)]);
    assert_eq!(base_turn(&both), deg(90), "the base keeps idle's turn");
    assert_eq!(arm_scale(&both), ONE, "sway's rest scale replaces idle's 0");
    assert_eq!(both[1].turn[1], deg(90));
    // the other way round, idle names both parts and takes both
    let back = m.layered(&[(Anim::Sway, 500), (Anim::Idle, 0)]);
    assert_eq!((base_turn(&back), arm_scale(&back)), (deg(90), 0));
    // a move the model lacks changes nothing
    assert_eq!(m.layered(&[(Anim::Jump, 10)]), vec![Pose::REST; 2]);
}

#[test]
fn a_child_moves_with_its_parent_and_scale_zero_hides_it() {
    let m = load(&jointed()).unwrap();
    let t = m.transforms(&m.poses(Some(Anim::Idle), 0));
    // the base turns 90°: the arm's tip (2, 0, 0) goes to (0, 0, -2)
    // with it, but the arm is scaled to nothing at its pivot (1, 0, 0),
    // which the base's turn carries to (0, 0, -1)
    assert!(near(t[1].apply(m.verts[3].0), v3(0, 0, -ONE)));
    assert!(near(t[1].apply(m.verts[4].0), v3(0, 0, -ONE)));
    // with no such animation, everything is at rest
    let rest = m.transforms(&m.poses(Some(Anim::Jump), 0));
    assert!(near(rest[1].apply(m.verts[3].0), v3(int(2), 0, 0)));
}

#[test]
fn an_anchor_follows_its_part() {
    let m = load(&jointed()).unwrap();
    let a = m.anchor("claw_left").unwrap();
    let t = m.transforms(&m.poses(Some(Anim::Sway), 500));
    let f = m.anchor_frame(a, &t);
    assert!(near(f.t, v3(ONE, 0, -ONE)));
    // and turns with it: the frame's +z now points along +x
    assert!(near(f.apply_dir(v3(0, 0, ONE)), v3(ONE, 0, 0)));
}

#[test]
fn the_player_slot_takes_the_players_colour_at_its_shade() {
    let m = load(&jointed()).unwrap();
    assert!(m.has_slot());
    assert!(!load(&tiny()).unwrap().has_slot());
    let c = m.paints([ONE, ONE / 2, 0]);
    // colours are listed by name: grey, then shell
    assert_eq!(c[0].rgb, [128 * ONE / 255; 3]);
    assert_eq!(c[1].rgb, [ONE / 2, ONE / 4, 0]);
}

#[test]
fn a_see_through_colour_is_drawn_apart_from_the_solid_ones() {
    let mut v = jointed();
    v["colours"]["shell"] = json!({ "slot": "player", "alpha": 40 });
    let m = load(&v).unwrap();
    let out = drawn(&m, &m.poses(None, 0));
    // the grey face solid, the shell's see-through at 40%
    assert_eq!((out.solid.len(), out.clear.len()), (33, 33));
    assert_eq!(out.clear[10], ONE * 40 / 100);
    assert_eq!(out.solid[10], ONE);
    // the object form without alpha is solid, as the array is
    v["colours"]["grey"] = json!({ "rgb": [128, 128, 128] });
    assert_eq!(load(&v).unwrap().swatches[0].alpha, ONE);
}

#[test]
fn models_come_in_gallery_order() {
    let mut crab = jointed();
    crab["kind"] = json!("crab");
    crab["anchors"]["claw_right"] = json!({ "part": "arm", "at": [2000, 0, 0] });
    crab["animations"] = crab_moves();
    let mut long = tiny();
    long["parts"][0]["vertices"][2] = json!([0, 0, 3000]);
    let all = json!({ "zz": tiny(), "aa": long, "crab": crab, "t": tile() });
    let ids: Vec<String> = parse_all(&all.to_string())
        .unwrap()
        .into_iter()
        .map(|m| m.id)
        .collect();
    // the crab first, then the tile (by kind), then the props shortest first
    assert_eq!(ids, ["crab", "t", "zz", "aa"]);
    assert_eq!(parse_all("{}"), Err("There are no models.".into()));
    let mut bad = tiny();
    bad["unit"] = json!("in");
    let e = parse_all(&json!({ "weed": bad }).to_string()).unwrap_err();
    assert!(e.starts_with("weed: the unit"), "{e}");
}

#[test]
fn a_colour_marked_glow_is_drawn_unlit_and_a_plain_one_lit() {
    let mut v = jointed();
    v["colours"]["grey"] = json!({ "rgb": [128, 128, 128], "glow": true });
    let m = load(&v).unwrap();
    let out = drawn(&m, &m.poses(None, 0)).solid;
    let glow = |tri: usize| out[tri * 3 * VERTEX_VALUES + 9];
    // the grey face glows (ONE), the slot's face does not
    assert_eq!((glow(0), glow(1)), (ONE, 0));
    v["colours"]["shell"] = json!({ "slot": "player", "glow": true });
    assert!(load(&v).unwrap().swatches[1].glow);
    v["colours"]["grey"] = json!({ "rgb": [300, 0, 0], "glow": true });
    refused(v, "the colour \"grey\" is outside 0 to 255.");
}

/// A board-like prop: one square, with a "3W" label of its own in white,
/// a colour no face uses.
fn stamped() -> Value {
    json!({
        "name": "Stamped",
        "kind": "board",
        "unit": "mm",
        "colours": { "sand": [235, 214, 166], "white": [255, 255, 255] },
        "parts": [{ "name": "slab", "vertices": [[-100, 40, -100], [100, 40, -100], [100, 40, 100], [-100, 40, 100]] }],
        "faces": [{ "colour": "sand", "v": [0, 1, 2, 3] }],
        "anchors": {
            "r0c0": { "part": "slab", "at": [0, 40, 0], "right": [1, 0, 0], "up": [0, 0, -1],
                      "size": [100, 60], "text": "3W", "ink": "white" }
        }
    })
}

#[test]
fn a_label_may_carry_its_own_text_in_an_ink_no_face_uses() {
    let m = load(&stamped()).unwrap();
    // white is the second colour by name (sand, white)
    assert_eq!(m.anchor("r0c0").unwrap().text, Some(("3W".to_string(), 1)));
    assert!(m.has_label("r0c0"));
    // a label without a text has none
    assert_eq!(load(&tile()).unwrap().anchor("letter").unwrap().text, None);
    // an unused colour that is no ink is still refused
    let mut v = stamped();
    v["anchors"]["r0c0"].as_object_mut().unwrap().remove("text");
    v["anchors"]["r0c0"].as_object_mut().unwrap().remove("ink");
    refused(v, "the colour \"white\" is on no face.");
}

#[test]
fn a_bad_fixed_text_is_refused_by_name() {
    let mut v = stamped();
    v["anchors"]["r0c0"]["text"] = json!("3w");
    refused(
        v,
        "the label \"r0c0\" has a text that is not capital letters and digits.",
    );
    let mut v = stamped();
    v["anchors"]["r0c0"]["ink"] = json!("gold");
    refused(v, "the label \"r0c0\" has no colour \"gold\" for its ink.");
    let mut v = stamped();
    v["anchors"]["r0c0"].as_object_mut().unwrap().remove("ink");
    refused(v, "the label \"r0c0\" needs both a text and an ink.");
    let mut v = jointed();
    v["anchors"]["claw_left"]["text"] = json!("A");
    v["anchors"]["claw_left"]["ink"] = json!("grey");
    refused(v, "the anchor \"claw_left\" has a text but is no label.");
}
