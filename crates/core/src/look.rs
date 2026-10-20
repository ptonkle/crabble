//! `data/look.json`: the presentation colours and the water's light the
//! core draws with (SECOND-ORDER R1.64). The players' colours by lobby
//! seat, the tiles' ink (plain, and green for a real word), the water
//! (fog, murk, light shafts and the caustic net on the sand, ported from
//! Honey Hole's underwater camera), and the Gallery's turntable.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::gfx::fx::{self, deg, ONE};
use crate::gfx::geom::v3;
use crate::gfx::render::{self, Air, Light, Paint, Rgb};

/// The players' colours by lobby seat (BRIEF.md: red, yellow, blue or
/// purple).
pub const PLAYER_COLOURS: [&str; 4] = ["red", "yellow", "blue", "purple"];

/// The tiles' ink: plain, and a real word's.
pub const INKS: [&str; 2] = ["plain", "real"];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// 0–255, by `PLAYER_COLOURS` name
    pub players: BTreeMap<String, [i32; 3]>,
    /// 0–255, by `INKS` name
    pub ink: BTreeMap<String, [i32; 3]>,
    pub water: Water,
    pub gallery: GalleryLook,
    pub marks: MarksLook,
    pub held: HeldLook,
    pub play: PlayLook,
    pub tally: TallyLook,
}

/// The tally's pacing, ms (`app/tally.rs`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TallyLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// from the end to the first line
    pub first_ms: i32,
    /// between one line and the next
    pub line_ms: i32,
    /// more between one player's column and the next
    pub gap_ms: i32,
    /// from the last line to the winner
    pub winner_ms: i32,
}

/// The game's screen: its camera's lens and fog, and the overhead view.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub fov_deg: i32,
    pub near_mm: i32,
    pub far_mm: i32,
    pub fog_mm: [i32; 2],
    pub overhead: OverheadLook,
}

/// The overhead view in the bottom-left corner.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverheadLook {
    /// its side, percent of the screen's height; while Tab is held
    pub size_pct: i32,
    pub big_pct: i32,
    /// its gap from the corner, percent of the screen's height
    pub margin_pct: i32,
    /// its camera's narrow field of view, and the room round the board
    pub fov_deg: i32,
    pub room_pct: i32,
    /// each crab's see-through circle
    pub circle_mm: i32,
    pub alpha_pct: i32,
    pub lift_mm: i32,
    pub sides: i32,
}

/// How a tile in a claw is drawn: smaller than on the board, so a 90 mm
/// crab can carry it (the person, CR R1.8: "let's shrink the tile
/// specifically while you're holding it so that the crab to tile ratio is
/// like A"). A placed tile and the ghost are always full size.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeldLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// a held tile's size, percent of a tile on the board
    pub tile_pct: i32,
    /// how long it takes to shrink when grabbed and to grow back when let go
    pub change_ms: i32,
    /// how far a held tile's face is tipped up from flat, back toward the
    /// camera behind the crab, so its letter reads (the person: "hold the
    /// tile up more at an angle such that it's easy to see what the tile is")
    pub tilt_deg: i32,
    /// how far each held tile is set out sideways from its claw, so two
    /// held tiles sit side by side rather than through each other
    pub spread_mm: i32,
}

impl HeldLook {
    /// A tile's size in percent, `ms` after it was grabbed (`held`) or let
    /// go (not `held`): it moves evenly between full size and `tile_pct`
    /// over `change_ms`.
    pub fn tile_pct_at(&self, held: bool, ms: i64) -> i32 {
        let t = ms.clamp(0, self.change_ms as i64);
        let d = (100 - self.tile_pct) as i64 * t / self.change_ms.max(1) as i64;
        if held {
            100 - d as i32
        } else {
            self.tile_pct + d as i32
        }
    }
}

/// The word marks' sizes (`crate::marks`), mm.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarksLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// strips round the tiles' feet, and on their tops
    pub board: MarkLie,
    pub tiles: MarkLie,
    /// the outer corners' radius
    pub radius_mm: i32,
    /// how far each mark sits above the one before
    pub step_mm: i32,
    /// about how long a dash is
    pub dash_mm: i32,
    /// the ghost tile's ring, 0-255
    pub ghost: [i32; 3],
}

/// Where a mark's strip lies: its height, and how far its inner edge is
/// from a square's middle and how wide it is.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkLie {
    pub y_mm: i32,
    pub inner_mm: i32,
    pub width_mm: i32,
    /// how far a skirt hangs from its outer edge (0: none)
    pub lip_mm: i32,
}

/// The water round the camera.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Water {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// the fog's colour is the murk's at eye level
    pub light: Light,
    /// the murk overhead and looking down, and the view's height
    /// (hundredths, -100 to 100) where each starts to show and is whole
    pub zenith: [i32; 3],
    pub zenith_band: [i32; 2],
    pub down: [i32; 3],
    pub down_band: [i32; 2],
    /// the light from the surface, for the shafts and the caustics
    pub sheen: [i32; 3],
    /// how fast the fog thickens from its start, a percent (k in
    /// (1 - e^(-k t)) / (1 - e^(-k))); 0 is an even S-curve
    pub fog_thickness_pct: i32,
    pub shafts: Shafts,
    pub caustics: Caustics,
}

/// Soft light shafts fanning down from the surface.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shafts {
    /// the light's direction, hundredths (toward it)
    pub dir: [i32; 3],
    /// two sets of stripes round it, so many to a radian each, drifting
    /// round once in so many milliseconds
    pub counts: [i32; 2],
    pub drift_ms: [i32; 2],
    /// strength and sharpness (a power), in hundredths, and the cosines
    /// (hundredths) from the light where they fade in and are whole
    pub strength_pct: i32,
    pub power_pct: i32,
    pub fade: [i32; 2],
}

/// The caustic net on what looks up.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Caustics {
    /// exactly three ripples
    pub ripples: [Ripple; 3],
    /// the lines' sharpness (a power) and their strength, percent
    pub power: i32,
    pub strength_pct: i32,
}

/// One ripple: its wavelength, heading (degrees from +x toward -z) and
/// period.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ripple {
    pub length_mm: i32,
    pub toward_deg: i32,
    pub period_ms: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GalleryLook {
    /// the sand the models stand on, and how far it reaches from the
    /// model, as a share of the camera's distance (percent)
    pub floor: [i32; 3],
    pub floor_pct: i32,
    /// one turn of the turntable, in seconds
    pub turn_seconds: i32,
    /// the camera's vertical field of view, and the room it leaves round a
    /// model (percent of the model's ball)
    pub fov_deg: i32,
    pub margin_pct: i32,
    /// the fog's start and end, percent of the camera's distance
    pub fog_pct: [i32; 2],
    /// each zoom step, percent nearer or further
    pub zoom_step_pct: i32,
    /// the closest and furthest zoom, percent of the fitted distance
    pub zoom_pct: [i32; 2],
}

fn colour_ok(c: &[i32; 3]) -> bool {
    c.iter().all(|v| (0..=255).contains(v))
}

/// Every name of `names` is in `map`, in range, and nothing else is.
fn named(what: &str, map: &BTreeMap<String, [i32; 3]>, names: &[&str]) -> Result<(), String> {
    for name in names {
        match map.get(*name) {
            None => return Err(format!("The look file has no {what} \"{name}\".")),
            Some(c) if !colour_ok(c) => return Err(format!("The {what} \"{name}\" is outside 0 to 255.")),
            Some(_) => {}
        }
    }
    if let Some(extra) = map.keys().find(|k| !names.contains(&k.as_str())) {
        return Err(format!("The look file has a {what} \"{extra}\" nothing uses."));
    }
    Ok(())
}

impl Look {
    pub fn parse(json: &str) -> Result<Look, String> {
        let look: Look =
            serde_json::from_str(json).map_err(|e| format!("The look file could not be read: {e}"))?;
        named("player colour", &look.players, &PLAYER_COLOURS)?;
        named("ink", &look.ink, &INKS)?;
        look.water.check()?;
        let g = &look.gallery;
        if !colour_ok(&g.floor) {
            return Err("The Gallery's floor is outside 0 to 255.".into());
        }
        if g.turn_seconds <= 0 {
            return Err("The Gallery's turn must take some seconds.".into());
        }
        if !(5..=120).contains(&g.fov_deg) {
            return Err("The Gallery's field of view is outside 5 to 120 degrees.".into());
        }
        if g.margin_pct < 100 || g.floor_pct <= 0 || g.fog_pct[0] >= g.fog_pct[1] || g.fog_pct[0] < 0 {
            return Err("The Gallery's margin, floor or fog is out of range.".into());
        }
        if !(1..=90).contains(&g.zoom_step_pct) || !(1..=100).contains(&g.zoom_pct[0]) || g.zoom_pct[1] < 100
        {
            return Err("The Gallery's zoom is out of range.".into());
        }
        let m = &look.marks;
        let lie_ok = |l: &MarkLie| {
            l.width_mm > 0
                && l.inner_mm > 0
                && l.width_mm <= m.radius_mm
                && m.radius_mm <= l.inner_mm + l.width_mm
                && (0..=l.y_mm).contains(&l.lip_mm)
        };
        if !lie_ok(&m.board) || !lie_ok(&m.tiles) || m.dash_mm <= 0 || m.step_mm < 0 || !colour_ok(&m.ghost) {
            return Err("The word marks' sizes are out of range.".into());
        }
        if !(10..=100).contains(&look.held.tile_pct)
            || !(1..=2000).contains(&look.held.change_ms)
            || !(0..=90).contains(&look.held.tilt_deg)
            || !(0..=200).contains(&look.held.spread_mm)
        {
            return Err("A held tile's size or change time is out of range.".into());
        }
        let p = &look.play;
        let o = &p.overhead;
        if !(5..=120).contains(&p.fov_deg)
            || !(1..p.far_mm).contains(&p.near_mm)
            || !(0 <= p.fog_mm[0] && p.fog_mm[0] < p.fog_mm[1])
            || !(5..=100).contains(&o.size_pct)
            || !(5..=100).contains(&o.big_pct)
            || !(0..=20).contains(&o.margin_pct)
            || !(1..=60).contains(&o.fov_deg)
            || o.room_pct < 100
            || o.circle_mm <= 0
            || !(1..=100).contains(&o.alpha_pct)
            || !(6..=64).contains(&o.sides)
        {
            return Err("The game screen's camera or overhead view is out of range.".into());
        }
        let t = &look.tally;
        if [t.first_ms, t.line_ms, t.gap_ms, t.winner_ms]
            .iter()
            .any(|ms| !(0..=10_000).contains(ms))
        {
            return Err("The tally's times are outside 0 to 10000 ms.".into());
        }
        Ok(look)
    }

    pub fn player(&self, slot: usize) -> Rgb {
        render::rgb(self.players[PLAYER_COLOURS[slot]])
    }

    /// A tile's ink by `INKS` index, solid and lit.
    pub fn ink(&self, k: usize) -> Paint {
        Paint::lit(render::rgb(self.ink[INKS[k]]))
    }
}

impl Water {
    fn check(&self) -> Result<(), String> {
        self.light.check()?;
        if ![self.zenith, self.down, self.sheen].iter().all(colour_ok) {
            return Err("A water colour is outside 0 to 255.".into());
        }
        let band = |b: [i32; 2]| (-100..=100).contains(&b[0]) && (-100..=100).contains(&b[1]) && b[0] < b[1];
        if !band(self.zenith_band) || !band(self.down_band) || !band(self.shafts.fade) {
            return Err("A water band is outside -100 to 100 or runs backwards.".into());
        }
        if self.shafts.dir == [0; 3] {
            return Err("The light shafts have no direction.".into());
        }
        if self.shafts.drift_ms.iter().any(|d| *d <= 0)
            || self
                .caustics
                .ripples
                .iter()
                .any(|r| r.length_mm <= 0 || r.period_ms <= 0)
        {
            return Err("A shaft's drift or a ripple's length or period is not above 0.".into());
        }
        if !(0..=1000).contains(&self.fog_thickness_pct) {
            return Err("The fog's thickness is outside 0 to 1000.".into());
        }
        Ok(())
    }

    /// The light and the water's air `clock_ms` in (the shafts drift and
    /// the ripples move with it).
    pub fn air(&self, clock_ms: u64) -> (Light, Air) {
        let hundredth = |v: i32| v * ONE / 100;
        // a phase going round once in `period` ms, in radians
        let phase = |period: i32| {
            let p = period.max(1) as u64;
            (fx::TWO_PI * (clock_ms % p) as i64 / p as i64) as i32
        };
        let s = &self.shafts;
        let dir = v3(hundredth(s.dir[0]), hundredth(s.dir[1]), hundredth(s.dir[2])).norm();
        let c = &self.caustics;
        let ripple = |r: &Ripple| {
            // radians per metre
            let k = (fx::TWO_PI * 1000 / r.length_mm.max(1) as i64) as i32;
            let a = deg(r.toward_deg);
            [
                fx::mul(k, fx::cos(a)),
                -fx::mul(k, fx::sin(a)),
                phase(r.period_ms),
                0,
            ]
        };
        let rgb = render::rgb;
        let air = Air {
            zenith: rgb(self.zenith),
            sky_band: self.zenith_band.map(hundredth),
            deep: rgb(self.down),
            deep_band: self.down_band.map(hundredth),
            shaft: [
                s.counts[0] * ONE,
                s.counts[1] * ONE,
                phase(s.drift_ms[0]),
                -phase(s.drift_ms[1]),
            ],
            shaft_look: [
                hundredth(s.strength_pct),
                hundredth(s.power_pct),
                hundredth(s.fade[0]),
                hundredth(s.fade[1]),
            ],
            shaft_dir: [dir.x, dir.y, dir.z],
            sheen: rgb(self.sheen),
            ripples: [
                ripple(&c.ripples[0]),
                ripple(&c.ripples[1]),
                ripple(&c.ripples[2]),
            ],
            caustic: [ONE / 3, c.power * ONE, hundredth(c.strength_pct), 0],
            fog_curve: hundredth(self.fog_thickness_pct),
        };
        (self.light.clone(), air)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// The shipped look file.
    pub fn shipped() -> String {
        include_str!("../../../data/look.json").to_string()
    }

    fn with(edit: impl Fn(&mut serde_json::Value)) -> Result<Look, String> {
        let mut v: serde_json::Value = serde_json::from_str(&shipped()).unwrap();
        edit(&mut v);
        Look::parse(&v.to_string())
    }

    #[test]
    fn the_shipped_look_is_accepted_with_four_seats_and_two_inks() {
        let look = Look::parse(&shipped()).unwrap();
        assert_eq!(look.players.len(), 4);
        assert_eq!(look.ink.len(), 2);
    }

    #[test]
    fn a_missing_or_extra_colour_is_refused_by_name() {
        let e = with(|v| {
            v["players"].as_object_mut().unwrap().remove("purple");
        });
        assert_eq!(e, Err("The look file has no player colour \"purple\".".into()));
        let e = with(|v| v["players"]["green"] = serde_json::json!([0, 200, 0]));
        assert_eq!(
            e,
            Err("The look file has a player colour \"green\" nothing uses.".into())
        );
        let e = with(|v| v["ink"]["real"] = serde_json::json!([0, 300, 0]));
        assert_eq!(e, Err("The ink \"real\" is outside 0 to 255.".into()));
        let e = with(|v| v["water"]["shafts"]["dir"] = serde_json::json!([0, 0, 0]));
        assert_eq!(e, Err("The light shafts have no direction.".into()));
        let e = with(|v| v["water"]["down_band"] = serde_json::json!([50, 10]));
        assert!(e.is_err());
        let e = with(|v| v["gallery"]["zoom_step_pct"] = serde_json::json!(0));
        assert_eq!(e, Err("The Gallery's zoom is out of range.".into()));
    }

    #[test]
    fn word_marks_wider_than_their_corners_are_refused() {
        // a strip wider than its corner's radius has no inner corner
        let e = with(|v| v["marks"]["tiles"]["width_mm"] = serde_json::json!(60));
        assert_eq!(e, Err("The word marks' sizes are out of range.".into()));
        let e = with(|v| v["marks"]["board"]["lip_mm"] = serde_json::json!(-1));
        assert_eq!(e, Err("The word marks' sizes are out of range.".into()));
        let e = with(|v| v["marks"]["ghost"] = serde_json::json!([0, 256, 0]));
        assert_eq!(e, Err("The word marks' sizes are out of range.".into()));
    }

    #[test]
    fn the_ripples_move_with_the_clock_and_come_round_in_their_period() {
        let look = Look::parse(&shipped()).unwrap();
        let w = &look.water;
        let period = w.caustics.ripples[0].period_ms as u64;
        let (_, a) = w.air(0);
        let (_, b) = w.air(period / 2);
        let (_, c) = w.air(period);
        assert_eq!(a.ripples[0][2], 0);
        // half a period is half way round: π
        assert!((b.ripples[0][2] - (fx::TWO_PI / 2) as i32).abs() <= 2);
        assert_eq!(c.ripples[0][2], 0);
        // a ripple 1 m long, heading along +x, is 2π a metre along x
        let r = Ripple {
            length_mm: 1000,
            toward_deg: 0,
            period_ms: 1000,
        };
        let mut w = w.clone();
        w.caustics.ripples[0] = r;
        let (_, d) = w.air(0);
        assert!((d.ripples[0][0] - fx::TWO_PI as i32).abs() <= 2 && d.ripples[0][1].abs() <= 2);
        // the caustics show on faces looking up, at the file's strength
        assert_eq!(d.caustic[2], w.caustics.strength_pct * ONE / 100);
    }

    #[test]
    fn a_held_tile_shrinks_to_its_size_and_grows_back_evenly() {
        let h = HeldLook {
            note: String::new(),
            tile_pct: 50,
            change_ms: 150,
            tilt_deg: 60,
            spread_mm: 35,
        };
        // grabbed: 100 at once, 75 half way, 50 from 150 ms on
        assert_eq!(h.tile_pct_at(true, 0), 100);
        assert_eq!(h.tile_pct_at(true, 75), 75);
        assert_eq!(h.tile_pct_at(true, 150), 50);
        assert_eq!(h.tile_pct_at(true, 5000), 50);
        // let go: back from 50 to 100 the same way
        assert_eq!(h.tile_pct_at(false, 0), 50);
        assert_eq!(h.tile_pct_at(false, 30), 60);
        assert_eq!(h.tile_pct_at(false, 150), 100);
        // the file's numbers are checked
        assert!(with(|v| v["held"]["tile_pct"] = serde_json::json!(0)).is_err());
        assert!(with(|v| v["held"]["change_ms"] = serde_json::json!(0)).is_err());
        assert!(with(|v| v["held"]["tilt_deg"] = serde_json::json!(91)).is_err());
    }
}
