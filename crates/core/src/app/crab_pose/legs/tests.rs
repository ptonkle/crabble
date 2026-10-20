use super::*;
use crate::model::{self, Model};

fn crab() -> Model {
    model::parse("crab", include_str!("../../../../../../data/models/crab.json")).unwrap()
}

fn look() -> LegsLook {
    crate::testkit::look().crab.legs
}

/// Whole millimetres between two points.
fn apart_mm(a: V3, b: V3) -> i32 {
    (a.sub(b).len() as i64 * 1000 / ONE as i64) as i32
}

#[test]
fn the_crab_has_eight_legs_in_two_alternating_fours() {
    let m = crab();
    let legs = legs_of(&m);
    assert_eq!(legs.len(), 8);
    // left 1 and 3 with right 2 and 4 (a real crab's alternating tetrapod)
    let group = |left: bool, n: usize| legs[if left { n - 1 } else { 3 + n }].group;
    let a = group(true, 1);
    assert!([group(true, 3), group(false, 2), group(false, 4)]
        .iter()
        .all(|g| *g == a));
    assert!([group(false, 1), group(true, 2), group(false, 3), group(true, 4)]
        .iter()
        .all(|g| *g != a));
    // the tips stand on the ground, out to each side
    for l in &legs {
        assert_eq!(l.tip.y, 0);
        assert_eq!(l.tip.x > 0, l.left);
    }
}

#[test]
fn a_leg_puts_its_tip_on_the_point_it_reaches_for() {
    let m = crab();
    let rest = m.transforms(&vec![crate::model::Pose::REST; m.parts.len()]);
    let body = rest[0];
    for l in legs_of(&m) {
        // its own rest place: unmoved (within a millimetre)
        let [_, _, f] = solve(&l, &body, l.tip);
        assert!(apart_mm(f.apply(l.tip), l.tip) <= 1);
        // up onto a tile edge 20 mm high, 10 mm further out and 8 mm forward
        let out = if l.left { 10 } else { -10 };
        let target = l.tip.add(model::mm([out, 20, 8]));
        let [h, k, f] = solve(&l, &body, target);
        assert!(apart_mm(f.apply(l.tip), target) <= 1, "the tip reaches it");
        // the knee stays above the hip (knee up) and the knee joint moves
        // with the hip's part
        assert!(h.apply(l.k).y > l.h.y);
        assert!(apart_mm(k.apply(l.k), h.apply(l.k)) <= 1);
    }
}

/// The feet of a crab facing +z at `x` (mm) on ground `ground`.
fn body_at(x_mm: i32, z_mm: i32, vel: V3, moving: bool, now: i64) -> Body {
    Body {
        place: Affine::translate(model::mm([x_mm, 0, z_mm])),
        vel,
        on_ground: true,
        moving,
        now,
    }
}

#[test]
fn a_foot_over_a_tile_stands_on_its_top_and_the_body_tilts_to_it() {
    let m = crab();
    let legs = legs_of(&m);
    let look = look();
    // a 20 mm step under the crab's left side (+x)
    let ground = |x: i32, _z: i32| if x > 0 { mm1(20) } else { 0 };
    let mut feet = Feet::default();
    let s = feet.update(&legs, &body_at(0, 0, V3::ZERO, false, 0), &ground, &look);
    for (l, tip) in legs.iter().zip(&s.tips) {
        assert_eq!(tip.y, if l.left { mm1(20) } else { 0 });
    }
    // the left side up: + about z lifts +x
    assert!(s.roll > 0);
    assert_eq!(s.pitch, 0);
    // the body rises toward the feet's mean (10 mm), follow_pct of the way
    assert_eq!(s.lift, mm1(10) * look.follow_pct / 100);
    assert_eq!(s.weight, ONE);
}

#[test]
fn walking_feet_stay_planted_and_step_in_alternating_fours() {
    let m = crab();
    let legs = legs_of(&m);
    let look = look();
    let ground = |_: i32, _: i32| 0;
    let mut feet = Feet::default();
    // sideways to its left at 394 mm a second (the game's speed), 16 ms a frame
    let speed = mm1(394);
    let mut steps = vec![0; 8];
    let mut last: Vec<Option<V3>> = vec![None; 8];
    for frame in 0..125 {
        let now = frame * 16;
        let x = (394 * now / 1000) as i32;
        let s = feet.update(&legs, &body_at(x, 0, v3(speed, 0, 0), true, now), &ground, &look);
        let up: Vec<bool> = feet.feet.iter().map(|f| f.step.is_some()).collect();
        for g in 0..2 {
            let a = legs.iter().zip(&up).any(|(l, u)| *u && l.group == g);
            let b = legs.iter().zip(&up).any(|(l, u)| *u && l.group != g);
            assert!(!(a && b), "frame {frame}: both fours in the air");
        }
        for j in 0..8 {
            match (up[j], last[j]) {
                // a planted foot does not slide
                (false, Some(p)) => assert_eq!(s.tips[j], p, "frame {frame}: foot {j} slid"),
                (true, _) => {}
                _ => {}
            }
            if up[j] && last[j].is_some() {
                steps[j] += 1;
            }
            last[j] = (!up[j]).then_some(s.tips[j]);
            // never far from its place under the body
            let home = Affine::translate(model::mm([x, 0, 0])).apply(legs[j].tip);
            assert!(apart_mm(s.tips[j], home) <= 3 * look.step_mm);
        }
    }
    // two seconds of scuttling: every foot stepped, often
    assert!(steps.iter().all(|n| *n >= 8), "steps {steps:?}");
}

#[test]
fn in_the_air_the_keyed_jump_has_the_legs_and_they_take_back_over_on_landing() {
    let m = crab();
    let legs = legs_of(&m);
    let look = look();
    let ground = |_: i32, _: i32| 0;
    let mut feet = Feet::default();
    let mut b = body_at(0, 0, V3::ZERO, false, 0);
    assert_eq!(feet.update(&legs, &b, &ground, &look).weight, ONE);
    b.on_ground = false;
    b.now = 16;
    feet.update(&legs, &b, &ground, &look);
    b.now = 16 + look.blend_ms as i64;
    assert_eq!(feet.update(&legs, &b, &ground, &look).weight, 0);
    b.on_ground = true;
    b.now = 1000;
    assert_eq!(feet.update(&legs, &b, &ground, &look).weight, 0, "just down");
    b.now = 1000 + look.blend_ms as i64;
    assert_eq!(feet.update(&legs, &b, &ground, &look).weight, ONE);
}
