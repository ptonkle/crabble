use super::*;
use crate::gfx::render::{self, VERTEX_VALUES};
use crate::testkit;
use serde_json::{json, Value};

fn one(things: Value) -> Value {
    json!({
        "name": "Test",
        "camera": { "target": [0, 0, 0], "yaw": 0, "pitch": 30, "distance": 2000 },
        "things": things
    })
}

fn load(v: &Value) -> Result<Scene, String> {
    parse("s", &v.to_string())
}

fn refused(v: Value, msg: &str) {
    assert_eq!(load(&v), Err(format!("s: {msg}")));
}

fn near(a: V3, b: V3) -> bool {
    a.sub(b).len() < ONE / 500
}

fn drawn(v: &Value) -> Tris {
    let s = load(v).unwrap();
    let models = testkit::models();
    let font = testkit::font();
    s.check(&models, &font).unwrap();
    let mut out = Tris::default();
    s.draw(&models, &font, &testkit::look(), 0, &mut out).unwrap();
    out
}

/// Each vertex's position and colour.
fn verts(list: &[i32]) -> Vec<(V3, [i32; 3])> {
    list.chunks(VERTEX_VALUES)
        .map(|v| (v3(v[0], v[1], v[2]), [v[6], v[7], v[8]]))
        .collect()
}

#[test]
fn a_camera_by_angle_stands_round_its_target() {
    let s = load(&one(json!([{ "model": "tile" }]))).unwrap();
    assert_eq!(s.name, "Test");
    // yaw 0 stands in front (+z), 30° up, 2 m out: (0, 1, 1.732)
    let (eye, fwd) = s.camera.eye_fwd();
    assert!(near(eye, v3(0, ONE, ONE * 1732 / 1000)), "{eye:?}");
    assert!(near(fwd, v3(0, -ONE / 2, -ONE * 866 / 1000)));
    assert_eq!(s.camera.fov, deg(FOV_DEG));
}

#[test]
fn a_camera_by_eye_becomes_the_same_orbit() {
    let mut v = one(json!([{ "model": "tile" }]));
    v["camera"] = json!({ "eye": [1000, 1000, 0], "target": [0, 0, 0], "fov": 30 });
    let c = load(&v).unwrap().camera;
    // from +x (yaw 90°), 45° up, √2 m away
    assert!((c.yaw - deg(90)).abs() <= deg(1) / 5, "{}", c.yaw);
    assert!((c.pitch - deg(45)).abs() <= deg(1) / 5);
    assert!((c.distance - ONE * 14142 / 10_000).abs() < 16);
    assert!(near(c.eye_fwd().0, v3(ONE, ONE, 0)));
    assert_eq!(c.fov, deg(30));
}

#[test]
fn a_bad_camera_or_thing_is_refused_by_name() {
    let mut v = one(json!([{ "model": "tile" }]));
    v["camera"] = json!({ "target": [0, 0, 0], "yaw": 0 });
    refused(v, "the camera needs an eye, or a yaw, pitch and distance.");
    let mut v = one(json!([{ "model": "tile" }]));
    v["camera"] = json!({ "target": [0, 0, 0], "eye": [0, 0, 0] });
    refused(v, "the camera's eye is at its target.");
    let mut v = one(json!([{ "model": "tile" }]));
    v["camera"]["pitch"] = json!(90);
    refused(v, "the camera's pitch is outside -89 to 89 degrees.");
    refused(one(json!([])), "the scene has no things.");
    refused(
        one(json!([{ "model": "crab", "colour": "green" }])),
        "thing 0 (crab): there is no colour \"green\".",
    );
    refused(
        one(json!([{ "model": "tile", "letter": "q" }])),
        "thing 0 (tile): \"q\" is not one capital letter.",
    );
    refused(
        one(json!([{ "model": "tile", "value": 3 }])),
        "thing 0 (tile): it has a value but no letter.",
    );
    refused(
        one(json!([{ "model": "tile", "face": "sideways" }])),
        "thing 0 (tile): there is no face \"sideways\".",
    );
    refused(
        one(json!([{ "model": "tile", "on": { "thing": 0, "anchor": "claw_left" } }])),
        "thing 0 (tile): it sits on thing 0, which is not listed before it.",
    );
    refused(
        one(json!([{ "model": "crab", "anim": "dance" }])),
        "thing 0 (crab): there is no animation \"dance\".",
    );
    refused(
        one(json!([{ "model": "tile", "alpha": 0 }])),
        "thing 0 (tile): its alpha is outside 1 to 100.",
    );
}

#[test]
fn a_scene_naming_a_missing_model_anchor_or_label_is_refused_against_the_models() {
    let models = testkit::models();
    let font = testkit::font();
    let check = |things: Value| load(&one(things)).unwrap().check(&models, &font);
    assert_eq!(
        check(json!([{ "model": "boat" }])),
        Err("s: thing 0 (boat): there is no such model.".into())
    );
    assert_eq!(
        check(json!([{ "model": "crab", "letter": "A" }])),
        Err("s: thing 0 (crab): the model has no letter label.".into())
    );
    assert_eq!(
        check(json!([{ "model": "tile", "anim": "jump" }])),
        Err("s: thing 0 (tile): the model has no animation \"jump\".".into())
    );
    assert_eq!(
        check(json!([{ "model": "crab" }, { "model": "tile", "on": { "thing": 0, "anchor": "mouth" } }])),
        Err("s: thing 1 (tile): thing 0 has no anchor \"mouth\".".into())
    );
    assert!(check(
        json!([{ "model": "crab" }, { "model": "tile", "on": { "thing": 0, "anchor": "claw_left" } }])
    )
    .is_ok());
}

#[test]
fn a_tile_shows_its_letter_and_value_in_the_ink_asked_for() {
    // the tile is two triangles; "Q" is one bar and "10" two: six more
    let out = drawn(&one(json!([{ "model": "tile", "letter": "Q", "ink": "real" }])));
    assert_eq!(render::vertex_count(&out.solid), 3 * (2 + 2 + 4));
    let look = testkit::look();
    let real = look.ink(1).rgb;
    let v = verts(&out.solid);
    assert!(
        v[6..].iter().all(|(_, c)| *c == real),
        "the letter and value are green"
    );
    // the letter sits 1 mm over the top (45 mm), in the middle
    assert!(v[6..12]
        .iter()
        .all(|(p, _)| (p.y - mm1(46)).abs() <= 1 && p.x.abs() <= mm1(10) + 1));
    // the value's two bars end at the value label's right edge (60 + 30 mm)
    let right = v[12..].iter().map(|(p, _)| p.x).max().unwrap();
    assert!((right - mm1(90)).abs() <= 2, "{right}");
    // with no ink named, plain; a letter's own value when none is given
    let s = load(&one(json!([{ "model": "tile", "letter": "E" }]))).unwrap();
    assert_eq!(s.things[0].dress.letter, Some(('E', 1)));
    assert_eq!(s.things[0].dress.ink, 0);
}

#[test]
fn a_tile_turned_face_down_lies_on_the_same_floor_with_its_letter_underneath() {
    let out = drawn(&one(json!([{ "model": "tile", "letter": "A", "face": "down" }])));
    let v = verts(&out.solid);
    // the slab still spans 0 to 45 mm
    let ys: Vec<i32> = v[..6].iter().map(|(p, _)| p.y).collect();
    assert!(ys.iter().all(|y| (-1..=mm1(45) + 1).contains(y)), "{ys:?}");
    // the letter is under it, 1 mm below the floor, facing down
    assert!(
        v[6..12].iter().all(|(p, _)| (p.y + mm1(1)).abs() <= 2),
        "{:?}",
        &v[6..12]
    );
    let n = &out.solid[6 * VERTEX_VALUES + 3..6 * VERTEX_VALUES + 6];
    assert_eq!(n[1], -ONE);
}

#[test]
fn a_see_through_tile_is_drawn_in_the_clear_list() {
    let out = drawn(&one(json!([{ "model": "tile", "letter": "S", "alpha": 40 }])));
    assert!(out.solid.is_empty());
    // the slab and the letter and value, all at 40%
    assert_eq!(render::vertex_count(&out.clear), 3 * 6);
    assert!(out.clear.chunks(VERTEX_VALUES).all(|v| v[10] == ONE * 40 / 100));
}

#[test]
fn a_tile_on_a_claw_moves_with_the_claw() {
    let things = |ms: u32| {
        json!([
            { "model": "crab", "colour": "blue", "anim": "grab_left", "ms": ms },
            { "model": "tile", "on": { "thing": 0, "anchor": "claw_left" } }
        ])
    };
    // at rest the claw's tip is 300 mm forward, 50 mm up: the tile's first
    // corner (-100, 0, -100) is there plus that
    let at_rest = verts(&drawn(&one(things(0))).solid);
    assert!(
        near(at_rest[6].0, v3(mm1(-100), mm1(50), mm1(200))),
        "{:?}",
        at_rest[6].0
    );
    // the crab takes the player's colour: blue
    assert_eq!(at_rest[0].1, testkit::look().player(2));
    // by 200 ms the claw has swung 90° toward +x: forward is now +x, and
    // the tile turns with it
    let swung = verts(&drawn(&one(things(200))).solid);
    let corner = Affine::rot_y(deg(90)).apply(mm([-100, 0, -100]));
    assert!(
        near(swung[6].0, v3(mm1(300), mm1(50), 0).add(corner)),
        "{:?}",
        swung[6].0
    );
}

#[test]
fn a_scene_moves_on_by_the_time_added() {
    let s = load(&one(json!([{ "model": "crab", "anim": "grab_left", "ms": 100 }]))).unwrap();
    let models = testkit::models();
    let font = testkit::font();
    let tip = |more: u32| {
        let mut out = Tris::default();
        s.draw(&models, &font, &testkit::look(), more, &mut out).unwrap();
        verts(&out.solid)[4].0
    };
    // 100 ms in is 45°; 100 more is 90°
    let h = ONE * 7071 / 10_000;
    assert!(near(
        tip(0),
        v3(fx::mul(h, mm1(300)), mm1(50), fx::mul(h, mm1(300)))
    ));
    assert!(near(tip(100), v3(mm1(300), mm1(50), 0)), "{:?}", tip(100));
}

#[test]
fn scenes_come_in_id_order_and_none_is_no_error() {
    let a = one(json!([{ "model": "tile" }]));
    let all = json!({ "b": a.clone(), "a": a });
    let ids: Vec<String> = parse_all(&all.to_string())
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(ids, ["a", "b"]);
    assert_eq!(parse_all("{}"), Ok(Vec::new()));
}


#[test]
fn a_label_with_its_own_text_always_shows_it_in_its_ink() {
    let board = json!({
        "name": "Board",
        "kind": "board",
        "unit": "mm",
        "colours": { "sand": [235, 214, 166], "white": [255, 255, 255] },
        "parts": [{ "name": "slab", "vertices": [[-100, 40, -100], [100, 40, -100], [100, 40, 100], [-100, 40, 100]] }],
        "faces": [{ "colour": "sand", "v": [0, 1, 2, 3] }],
        "anchors": {
            "r0c0": { "part": "slab", "at": [0, 40, 0], "right": [1, 0, 0], "up": [0, 0, -1],
                      "size": [100, 60], "text": "3W", "ink": "white" }
        }
    });
    let mut models = testkit::models();
    models.push(crate::model::parse("board", &board.to_string()).unwrap());
    let s = load(&one(json!([{ "model": "board" }]))).unwrap();
    let font = testkit::font();
    s.check(&models, &font).unwrap();
    let mut out = Tris::default();
    s.draw(&models, &font, &testkit::look(), 0, &mut out).unwrap();
    // the square is two triangles; "3" and "W" one bar each: four more
    assert_eq!(render::vertex_count(&out.solid), 3 * (2 + 4));
    let v = verts(&out.solid);
    assert!(v[6..].iter().all(|(_, c)| *c == render::rgb([255, 255, 255])), "white text");
    // 1 mm over the top (40 mm)
    assert!(v[6..].iter().all(|(p, _)| (p.y - mm1(41)).abs() <= 1));
}
