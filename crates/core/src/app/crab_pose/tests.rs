use super::*;
use crate::gfx::geom::V3;
use crate::sim::crab::Driver;

/// Seven parts: the body, the eyes, the arms and the fingers (left first).
const PARTS: Parts = Parts {
    body: 0,
    eye: [Some(1), Some(2)],
    arm: [Some(3), Some(4)],
    finger: [Some(5), Some(6)],
};

/// One metre a second, and a jump's take-off at two.
const SPEED: i32 = ONE;
const JUMP: i32 = 2 * ONE;

fn look() -> CrabLook {
    crate::testkit::look().crab
}

fn cx(now: i64) -> Context {
    Context {
        now,
        seat: 0,
        speed: SPEED,
        jump_speed: JUMP,
        cycle_ms: 480,
    }
}

/// A crab facing +z (yaw 0), standing still on the ground.
fn crab() -> Crab {
    Crab::new(Driver::Local, V3::ZERO, 0, 0)
}

fn rest() -> Vec<Pose> {
    vec![Pose::REST; 7]
}

fn degrees(a: i32) -> i32 {
    (a as i64 * 360 / TURN as i64) as i32
}

#[test]
fn a_scuttling_crab_leans_its_top_toward_where_it_goes() {
    let look = look();
    let mut rig = Rig::default();
    let mut c = crab();
    // to its left (+x, facing +z) at full speed
    c.vel = v3(SPEED, 0, 0);
    c.gait = Gait::Left;
    for k in 0..60 {
        rig.update(&c, 0, &cx(k * 16), &look);
    }
    // a roll tipping the top toward +x is negative about z; after ten time
    // constants the lean is all of lean_deg
    let roll = degrees(rig.lean[0]);
    assert!((roll + look.lean_deg).abs() <= 1, "a roll of {roll} degrees");
    let mut poses = rest();
    rig.dress(&c, 0, &mut poses, &PARTS, &cx(60 * 16), &look);
    assert_eq!(poses[0].turn[2], rig.lean[0], "the body takes the lean");
    // the eye stalks trail: tipped the other way (+ about z tips toward -x)
    assert!(poses[1].turn[2] > 0 && poses[2].turn[2] > 0);
    // to its right, the other way
    c.vel = v3(-SPEED, 0, 0);
    c.gait = Gait::Right;
    for k in 60..120 {
        rig.update(&c, 0, &cx(k * 16), &look);
    }
    assert!((degrees(rig.lean[0]) - look.lean_deg).abs() <= 1);
}

#[test]
fn a_landing_squashes_the_crab_and_then_it_stands_even() {
    let look = look();
    let mut rig = Rig::default();
    let mut c = crab();
    rig.update(&c, 0, &cx(0), &look);
    // in the air, coming down as fast as it went up
    c.on_ground = false;
    c.air_ms = 1000;
    c.vel = v3(0, -JUMP, 0);
    rig.update(&c, 0, &cx(16), &look);
    // down
    c.on_ground = true;
    c.air_ms = 0;
    c.vel = V3::ZERO;
    rig.update(&c, 0, &cx(32), &look);
    let s = rig.dress(&c, 0, &mut rest(), &PARTS, &cx(32), &look);
    // a full jump's fall: squash_pct shorter, half as much wider
    let pct = |q: i32| ((q as i64 * 100 + ONE as i64 / 2) / ONE as i64) as i32;
    assert_eq!(pct(s.y), 100 - look.squash_pct);
    assert_eq!(pct(s.x), 100 + look.squash_pct / 2);
    assert_eq!(s.x, s.z);
    // a third of the way through, the rebound starts; at the end it is even
    let later = rig.dress(
        &c,
        0,
        &mut rest(),
        &PARTS,
        &cx(32 + look.squash_ms as i64 / 2),
        &look,
    );
    assert!(later.y > ONE, "it rebounds past even");
    let done = rig.dress(&c, 0, &mut rest(), &PARTS, &cx(32 + look.squash_ms as i64), &look);
    assert_eq!(done, v3(ONE, ONE, ONE));
}

#[test]
fn leaving_the_ground_stretches_the_crab() {
    let look = look();
    let mut rig = Rig::default();
    let mut c = crab();
    rig.update(&c, 0, &cx(0), &look);
    c.on_ground = false;
    c.vel = v3(0, JUMP, 0);
    let s = rig.dress(&c, 0, &mut rest(), &PARTS, &cx(16), &look);
    assert_eq!(s.y, ONE + look.stretch_pct * ONE / 100);
    c.air_ms = look.stretch_ms as u32;
    let s = rig.dress(&c, 0, &mut rest(), &PARTS, &cx(16), &look);
    assert_eq!(s, v3(ONE, ONE, ONE), "eased back by stretch_ms");
}

#[test]
fn a_crab_standing_still_fidgets_only_after_a_while() {
    let look = look();
    let mut rig = Rig::default();
    let c = crab();
    rig.update(&c, 0, &cx(0), &look);
    let wait = look.fidget_after_ms as i64;
    // just short of the wait: nothing past the keyed moves
    rig.update(&c, 0, &cx(wait - 1), &look);
    let mut poses = rest();
    rig.dress(&c, 0, &mut poses, &PARTS, &cx(wait - 1), &look);
    assert_eq!(poses, rest());
    // a quarter into the first fidget something moves
    let at = wait + look.fidget_ms as i64 / 4;
    rig.update(&c, 0, &cx(at), &look);
    let mut poses = rest();
    rig.dress(&c, 0, &mut poses, &PARTS, &cx(at), &look);
    assert_ne!(poses, rest());
}

#[test]
fn a_claw_squeezes_after_a_grab_and_clacks_after_a_place() {
    let look = look();
    let mut rig = Rig::default();
    let mut c = crab();
    rig.update(&c, 0, &cx(0), &look);
    c.reach[0] = Some((Reach::Grab, 1000));
    let finger_at = |rig: &Rig, c: &Crab, now: i64| {
        let mut poses = rest();
        rig.dress(c, 0, &mut poses, &PARTS, &cx(now), &look);
        (poses[5].turn[0], poses[6].turn[0])
    };
    // half way through the squeeze (220 ms on, when the claw has closed)
    let mid = 1000 + 220 + look.snap_ms as i64 / 2;
    let (l, r) = finger_at(&rig, &c, mid);
    assert_eq!(l, deg(look.snap_deg) / 2, "closes harder (+ about x)");
    assert_eq!(r, 0, "the other claw is still");
    assert_eq!(finger_at(&rig, &c, 1000 + 219), (0, 0));
    // a place: two clacks open (- about x) after its 440 ms
    c.reach[1] = Some((Reach::Place, 5000));
    let first = 5000 + 440 + look.snap_ms as i64 / 2;
    assert_eq!(finger_at(&rig, &c, first).1, -deg(look.snap_deg));
    assert_eq!(
        finger_at(&rig, &c, 5000 + 440 + look.snap_ms as i64).1,
        0,
        "shut between"
    );
    assert!(
        finger_at(&rig, &c, first + look.snap_ms as i64).1 < 0,
        "and again"
    );
}
