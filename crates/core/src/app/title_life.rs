//! The life about the title card (F4, SECOND-ORDER R11.*): seat-colour
//! crabs scuttling sideways across the sand, one carrying a tile that it
//! sets down and picks up again, one peeking out from behind the card, and
//! fish swimming by behind it. Every crab is posed through the game's own
//! [`pose_crab`] (its walk, idle, hold, grab and place moves, and the
//! extra motion of `app/crab_pose.rs`: the lean and sway of a scuttle, a
//! fidget at rest), so a change
//! to how the game's crabs move shows here too. Where each goes and when
//! is `data/look.json`'s `title.crabs` and `title.fish`; everything follows
//! from the clock, so a still at a given time always comes out the same.

use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::{v3, yaw_of, Affine, V3};
use crate::gfx::render::Tris;
use crate::glyph::Font;
use crate::letters;
use crate::look::{Look, TitleCrab, TitleFish, PLAYER_COLOURS};
use crate::model::{mm, mm1, Anim, Model};
use crate::scene::{self, Dress};
use crate::sim::{Gait, Reach};

use super::crab_pose::{lean_to, CrabLook, Extra, Motion};
use super::view3d::{anim_ms, blend, gait_body, in_claw, pose_crab, Doing};

/// The fish model of the title card.
pub const FISH: &str = "title-fish";

/// Where a title crab's tile is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carried {
    /// in its right claw, flown in from the sand `since` ms ago
    Claw { since: u32 },
    /// lying on the sand
    Sand,
    /// set down `since` ms ago, flying from the claw to the sand
    Down { since: u32 },
}

/// A title crab `ms` into the clock: where it stands (Q16 metres), the way
/// it faces, what it is doing, where its tile is (if it has one) and the
/// sand pose's spot: the end it rests at, or last rested at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Walker {
    pub pos: V3,
    pub yaw: i32,
    pub gait: Gait,
    /// walking, ms of the game's walk (so the legs keep pace with the sand)
    pub walk_ms: u32,
    pub reach: Option<(Reach, u32)>,
    pub tile: Option<Carried>,
    /// the end its tile lies at, or was last set down at
    pub end: V3,
    /// resting, how long it has stood still (ms; 0 walking)
    pub still_ms: i64,
}

/// A title crab `ms` into the clock. `walk_mm_per_s` is the game's walking
/// speed, so its walk move plays at the pace of its steps; `place_ms` and
/// `grab_ms` are the crab model's place and grab moves' lengths.
///
/// Its round: from `from` to `to`, a rest at `to`, back, a rest at `from`.
/// It faces the camera's side of its path (+z) and scuttles sideways. With
/// a letter, it holds the tile while it walks; at each end it sets it down
/// as it arrives and picks it up as it leaves.
pub fn walker(c: &TitleCrab, ms: i64, walk_mm_per_s: i32, place_ms: u32, grab_ms: u32) -> Walker {
    let a = mm([c.from[0], 0, c.from[1]]);
    let b = mm([c.to[0], 0, c.to[1]]);
    let d = b.sub(a);
    // one way, ms
    let leg = (d.len() as i64 * 1000 / mm1(c.speed_mm_per_s).max(1) as i64).max(1);
    let [r0, r1] = c.rest_ms.map(|r| r.max(0) as i64);
    let round = 2 * leg + r0 + r1;
    let t = (ms + c.start_ms as i64).rem_euclid(round);
    // facing across its path, toward the camera's side
    let s = if d.x > 0 || (d.x == 0 && d.z < 0) { 1 } else { -1 };
    let yaw = yaw_of(v3(-d.z * s, 0, d.x * s));
    // a step toward `to` is a step to its left (sim::crab::walk_velocity)
    let (out, back) = if s > 0 {
        (Gait::Left, Gait::Right)
    } else {
        (Gait::Right, Gait::Left)
    };
    let steps = |tw: i64| (tw * c.speed_mm_per_s as i64 / walk_mm_per_s.max(1) as i64) as u32;
    let has = c.letter.is_some();
    // a rest `tr` into `len` at `end`: set down, lying, picked up
    let rest = |tr: i64, len: i64, end: V3| {
        let up_at = (len - grab_ms as i64).max(place_ms as i64);
        let (reach, tile) = if tr < place_ms as i64 {
            (
                Some((Reach::Place, tr as u32)),
                Carried::Down { since: tr as u32 },
            )
        } else if tr >= up_at {
            let since = (tr - up_at) as u32;
            (Some((Reach::Grab, since)), Carried::Claw { since })
        } else {
            (Some((Reach::Place, tr as u32)), Carried::Sand)
        };
        Walker {
            pos: end,
            yaw,
            gait: Gait::Still,
            walk_ms: 0,
            reach: reach.filter(|_| has),
            tile: has.then_some(tile),
            end,
            still_ms: tr,
        }
    };
    let walking = |from: V3, to: V3, tw: i64, gait: Gait| Walker {
        pos: from.lerp(to, (tw * ONE as i64 / leg) as i32),
        yaw,
        gait,
        walk_ms: steps(tw),
        // the grab as it set off may still be closing
        reach: has.then_some((Reach::Grab, (tw + grab_ms as i64) as u32)),
        tile: has.then_some(Carried::Claw {
            since: (tw + grab_ms as i64) as u32,
        }),
        end: from,
        still_ms: 0,
    };
    if t < leg {
        walking(a, b, t, out)
    } else if t < leg + r1 {
        rest(t - leg, r1, b)
    } else if t < 2 * leg + r1 {
        walking(b, a, t - leg - r1, back)
    } else {
        rest(t - 2 * leg - r1, r0, a)
    }
}

/// A title fish `ms` into the clock: where it is (Q16 metres) and the way
/// it heads, or `None` while it is out of sight between swims.
pub fn fish_at(f: &TitleFish, ms: i64) -> Option<(V3, i32)> {
    let round = f.swim_ms as i64 + f.rest_ms.max(0) as i64;
    let t = (ms + f.start_ms as i64).rem_euclid(round.max(1));
    if t >= f.swim_ms as i64 {
        return None;
    }
    let (a, b) = (mm(f.from), mm(f.to));
    let at = a.lerp(b, (t * ONE as i64 / f.swim_ms as i64) as i32);
    Some((at, yaw_of(b.sub(a))))
}

/// What the game's extra motion (`app/crab_pose.rs`) reads of a title
/// crab, from the clock alone: a steady scuttle's lean and sway while it
/// walks, a fidget while it rests. It never jumps or turns.
fn motion(
    c: &TitleCrab,
    w: &Walker,
    seat: usize,
    walk_mm_per_s: i32,
    side_ms: u32,
    look: &CrabLook,
) -> Motion {
    let speed = fx::ratio(c.speed_mm_per_s, walk_mm_per_s.max(1)).min(ONE);
    let side = match w.gait {
        Gait::Left => speed,
        Gait::Right => -speed,
        _ => 0,
    };
    Motion {
        seat,
        on_ground: true,
        walking: w.gait != Gait::Still,
        side,
        moving: side.abs(),
        lean: lean_to(side, 0, 0, look),
        phase: (w.walk_ms as i64 * fx::TURN as i64 / side_ms.max(1) as i64) as i32,
        still_ms: w.still_ms,
        ..Motion::default()
    }
}

/// A sine wave's value (Q16) at `ms` into a cycle of `period_ms`.
fn wave(ms: i64, period_ms: i64) -> i32 {
    fx::sin((ms.rem_euclid(period_ms) * fx::TURN as i64 / period_ms) as i32)
}

/// Draws the title card's crabs and fish `ms` into the clock.
pub fn draw(models: &[Model], font: &Font, look: &Look, ms: i64, walk_mm_per_s: i32, out: &mut Tris) {
    let model = |id: &str| models.iter().find(|m| m.id == id);
    if let (Some(crab), Some(tm)) = (model("crab"), model("tile")) {
        let place_ms = anim_ms(crab, Anim::PlaceRight);
        let grab_ms = anim_ms(crab, Anim::GrabRight);
        let change = look.held.change_ms as i64;
        let side_ms = anim_ms(crab, Anim::WalkSide);
        for (seat, c) in look.title.crabs.iter().enumerate() {
            let w = walker(c, ms, walk_mm_per_s, place_ms, grab_ms);
            let now = (ms + c.start_ms as i64).max(0) as u32;
            let holds = matches!(w.tile, Some(Carried::Claw { .. }));
            let doing = Doing {
                body: gait_body(crab, w.gait, w.walk_ms, now),
                now,
                holds: [false, holds],
                reach: [None, w.reach],
                extra: Some(Extra {
                    motion: motion(c, &w, seat, walk_mm_per_s, side_ms, &look.crab),
                    look: &look.crab,
                    legs: None,
                }),
            };
            let transforms = pose_crab(crab, &doing);
            let place = Affine::translate(w.pos).then(&Affine::rot_y(w.yaw));
            let dress = Dress {
                colour: PLAYER_COLOURS.iter().position(|p| *p == c.colour).unwrap_or(0),
                ..Default::default()
            };
            let _ = scene::draw_dressed(crab, &transforms, &place, &dress, look, font, out);
            let (Some(letter), Some(tile)) = (c.letter, w.tile) else {
                continue;
            };
            // on the sand: flat, face up, ahead of its claws and a little
            // to its right (the camera's left, clear of the crab), its
            // letter the right way up to the camera
            let ahead = Affine::translate(w.end)
                .then(&Affine::rot_y(w.yaw))
                .then(&Affine::translate(mm([-110, 0, 190])))
                .then(&Affine::rot_y(deg(12)));
            let sand = ahead.then(&scene::placing(tm, V3::ZERO, [0; 3], ONE, false));
            let claw = |pct: i32| {
                in_claw(
                    look,
                    tm,
                    (crab, &transforms, &place),
                    crate::sim::tiles::RIGHT,
                    pct,
                    0,
                )
            };
            let h = &look.held;
            let pose = match tile {
                Carried::Sand => Some(sand),
                Carried::Claw { since } => {
                    claw(h.tile_pct_at(true, since as i64)).map(|to| blend(&sand, &to, since as i64, change))
                }
                Carried::Down { since } => claw(h.tile_pct_at(false, since as i64))
                    .map(|from| blend(&from, &sand, since as i64, change)),
            };
            let Some(pose) = pose else {
                continue;
            };
            let tdress = Dress {
                letter: Some((letter, letters::value(letter).unwrap_or(0))),
                ..Default::default()
            };
            let tt = tm.transforms(&tm.poses(None, 0));
            let _ = scene::draw_dressed(tm, &tt, &pose, &tdress, look, font, out);
        }
    }
    if let Some(fm) = model(FISH) {
        for f in &look.title.fish {
            let Some((at, yaw)) = fish_at(f, ms) else {
                continue;
            };
            // a gentle rise and fall, and a wag of the heading with the tail
            let at = at.add(v3(0, fx::mul(wave(ms + f.start_ms as i64, 3100), mm1(18)), 0));
            let yaw = yaw + fx::mul(wave(ms, 900), deg(4));
            let transforms =
                fm.transforms(&fm.poses(Some(Anim::Sway), (ms + f.start_ms as i64).max(0) as u32));
            let place = scene::placing(fm, at, [0, yaw, 0], f.scale_pct * ONE / 100, false);
            let _ = scene::draw_dressed(fm, &transforms, &place, &Dress::default(), look, font, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// From (0, 0) to (1000, 0) at 500 mm a second (2 s a way), resting
    /// 1 s at `from` and 3 s at `to`: an 8 s round.
    fn crab(letter: Option<char>) -> TitleCrab {
        TitleCrab {
            colour: "red".into(),
            from: [0, 0],
            to: [1000, 0],
            speed_mm_per_s: 500,
            rest_ms: [1000, 3000],
            start_ms: 0,
            letter,
        }
    }

    /// Within a millimetre.
    fn near(a: i32, mm: i32) -> bool {
        (a as i64 * 1000 / ONE as i64 - mm as i64).abs() <= 1
    }

    #[test]
    fn a_title_crab_scuttles_out_rests_and_scuttles_back() {
        // the game walks at 250 mm a second: twice the steps a second
        let at = |ms| walker(&crab(None), ms, 250, 400, 400);
        // half way out (1 s): 500 mm along, its walk 2000 ms in, to its left
        let w = at(1000);
        assert!(near(w.pos.x, 500) && near(w.pos.z, 0));
        assert_eq!(w.walk_ms, 2000);
        assert_eq!(w.gait, Gait::Left);
        // facing +z, toward the camera: yaw 0
        assert_eq!(w.yaw, 0);
        // resting at `to` from 2 s to 5 s
        let w = at(3000);
        assert!(near(w.pos.x, 1000));
        assert_eq!(w.gait, Gait::Still);
        assert_eq!(w.tile, None, "no letter, no tile");
        // back the other way: a quarter of the way (5.5 s) is 750 mm
        let w = at(5500);
        assert!(near(w.pos.x, 750));
        assert_eq!(w.gait, Gait::Right);
        // resting at `from` at 7.5 s, and round again at 8 s
        assert!(near(at(7500).pos.x, 0));
        assert_eq!(at(7500).gait, Gait::Still);
        assert!(near(at(9000).pos.x, 500));
    }

    #[test]
    fn a_title_crab_with_a_letter_sets_its_tile_down_and_picks_it_up() {
        // place 400 ms, grab 400 ms
        let at = |ms| walker(&crab(Some('S')), ms, 500, 400, 400);
        // walking out: in its claw
        assert!(matches!(at(1000).tile, Some(Carried::Claw { .. })));
        // arrived at 2 s: setting it down 100 ms in
        assert_eq!(at(2100).tile, Some(Carried::Down { since: 100 }));
        assert_eq!(at(2100).reach, Some((Reach::Place, 100)));
        // lying on the sand at `to` through the rest
        let w = at(3500);
        assert_eq!(w.tile, Some(Carried::Sand));
        assert!(near(w.end.x, 1000));
        // picked up 400 ms before it sets off at 5 s: 200 ms in at 4.8 s
        assert_eq!(at(4800).tile, Some(Carried::Claw { since: 200 }));
        assert_eq!(at(4800).reach, Some((Reach::Grab, 200)));
        // the rest at `from` is 1 s: set down, then picked up at 7.6 s
        assert_eq!(at(7300).tile, Some(Carried::Down { since: 300 }));
        assert_eq!(at(7500).tile, Some(Carried::Sand));
        assert_eq!(at(7700).tile, Some(Carried::Claw { since: 100 }));
    }

    #[test]
    fn a_title_crab_going_the_other_way_still_faces_the_camera() {
        // from (1000, 0) to (0, 0): facing +z still (yaw 0); a step toward
        // `to` (-x) is to its right
        let mut c = crab(None);
        c.from = [1000, 0];
        c.to = [0, 0];
        let w = walker(&c, 1000, 500, 400, 400);
        assert_eq!(w.yaw, 0);
        assert_eq!(w.gait, Gait::Right);
    }

    #[test]
    fn a_title_fish_swims_across_then_is_out_of_sight() {
        let f = TitleFish {
            from: [-1000, 500, 0],
            to: [1000, 500, 0],
            swim_ms: 4000,
            rest_ms: 2000,
            start_ms: 0,
            scale_pct: 100,
        };
        let (at, yaw) = fish_at(&f, 1000).unwrap();
        assert!(near(at.x, -500) && near(at.y, 500));
        // heading +x: a quarter turn
        assert_eq!(yaw, fx::QUARTER);
        assert!(fish_at(&f, 5000).is_none());
        assert!(near(fish_at(&f, 6000).unwrap().0.x, -1000));
    }
}
