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
    pub title: TitleLook,
    pub touch: TouchLook,
    /// a crab's motion past its keyed moves (`app/crab_pose.rs`)
    pub crab: crate::app::crab_pose::CrabLook,
    pub colourblind: ColourblindLook,
}

/// Colourblind mode (Settings; the person, F16: "add colorblind mode with
/// enhanced visbility"; SECOND-ORDER R7.6): seat colours told apart under
/// the common colour blindnesses, a real word's ink apart from the plain
/// one by lightness, the board's premium squares in stronger colours,
/// outlines `thick_pct` percent as wide, and a symbol per seat on each
/// crab and overhead circle.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColourblindLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// 0-255, by `PLAYER_COLOURS` name
    pub players: BTreeMap<String, [i32; 3]>,
    /// 0-255, by `INKS` name
    pub ink: BTreeMap<String, [i32; 3]>,
    /// 0-255, by the board model's colour names
    pub board: BTreeMap<String, [i32; 3]>,
    pub thick_pct: i32,
    /// a crab's symbol: how wide, and how high over the crab's feet (mm)
    pub symbol_mm: i32,
    pub symbol_up_mm: i32,
    /// the symbols' ink, and the dark rim round a crab's
    pub symbol_ink: [i32; 3],
    pub symbol_rim: [i32; 3],
}

impl Look {
    /// This look as colourblind mode draws it: its colourblind seat
    /// colours and inks in place of the usual ones.
    pub fn colourblind_look(&self) -> Look {
        Look {
            players: self.colourblind.players.clone(),
            ink: self.colourblind.ink.clone(),
            ..self.clone()
        }
    }
}

/// Each seat's symbol in colourblind mode, by seat: a triangle, a
/// square, a plus and a star.
pub const SYMBOLS: [&str; 4] = ["triangle", "square", "plus", "star"];

/// Seat `seat`'s symbol as a polygon round its middle, every corner seen
/// from the middle (so it draws as a fan from there), thousandths of its
/// half width, anticlockwise from straight up.
pub fn symbol(seat: usize) -> Vec<(i32, i32)> {
    let ring = |n: i32, r: &dyn Fn(i32) -> i32| -> Vec<(i32, i32)> {
        (0..n)
            .map(|k| {
                let a = crate::gfx::fx::TURN / n * k;
                let r = r(k);
                // x to the right, y up: straight up at k = 0
                (
                    -((crate::gfx::fx::sin(a) as i64 * r as i64) >> 16) as i32,
                    ((crate::gfx::fx::cos(a) as i64 * r as i64) >> 16) as i32,
                )
            })
            .collect()
    };
    match seat % SYMBOLS.len() {
        0 => ring(3, &|_| 1000),
        1 => vec![(-760, 760), (-760, -760), (760, -760), (760, 760)],
        2 => vec![
            (-300, 1000),
            (-300, 300),
            (-1000, 300),
            (-1000, -300),
            (-300, -300),
            (-300, -1000),
            (300, -1000),
            (300, -300),
            (1000, -300),
            (1000, 300),
            (300, 300),
            (300, 1000),
        ],
        _ => ring(10, &|k| if k % 2 == 0 { 1000 } else { 420 }),
    }
}

/// The touch controls on a phone or tablet (`app/touch.rs`), in CSS
/// pixels for a phone whose short side is `short_px`; a bigger screen
/// scales them up to `scale_max_pct`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TouchLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    pub short_px: i32,
    pub scale_max_pct: i32,
    /// each claw's button across, the jump button's, and the small ones'
    /// (pause)
    pub claw_px: i32,
    pub jump_px: i32,
    pub small_px: i32,
    /// from the screen's edges, and between buttons
    pub margin_px: i32,
    pub gap_px: i32,
    /// the thumbstick's ring across
    pub stick_px: i32,
    /// how far the stick must lean, percent of its ring's radius, to walk
    /// that way
    pub dead_pct: i32,
    /// a drag's turn, percent of the mouse's for the same pixels
    pub look_pct: i32,
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

/// The title card behind the Title menu (`app/title.rs`): its camera
/// tipped down so the card sits high, over the menu, and its idle motion.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitleLook {
    #[serde(rename = "_note", default)]
    pub note: String,
    /// how far the camera looks down past the scene's own camera, so the
    /// card rises up the screen
    pub tilt_deg: i32,
    /// each tile bobs this far up and down, rocking this many degrees,
    /// once every `bob_ms`
    pub bob_mm: i32,
    pub rock_deg: i32,
    pub bob_ms: i32,
    /// each bubble rises this far once every `rise_ms`, swaying this far
    /// side to side, growing in and shrinking away at the ends
    pub rise_mm: i32,
    pub rise_ms: i32,
    pub sway_mm: i32,
    /// the camera drifts this many degrees round and back once every
    /// `drift_ms`
    pub drift_deg: i32,
    pub drift_ms: i32,
    #[serde(rename = "_crabs", default)]
    pub crabs_note: String,
    /// the crabs about the card (F4, SECOND-ORDER R11.1)
    pub crabs: Vec<TitleCrab>,
    /// the fish that swim by behind it
    pub fish: Vec<TitleFish>,
}

/// A crab about the title card: it scuttles sideways from `from` to `to`
/// (x and z on the sand, mm) and back, resting `rest_ms` at each end (at
/// `from`, at `to`), `start_ms` into its round at the clock's 0. With a
/// `letter` it carries that tile, setting it down on the sand at each end
/// and picking it up again before it sets off.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitleCrab {
    /// one of `PLAYER_COLOURS`
    pub colour: String,
    pub from: [i32; 2],
    pub to: [i32; 2],
    pub speed_mm_per_s: i32,
    pub rest_ms: [i32; 2],
    #[serde(default)]
    pub start_ms: i32,
    #[serde(default)]
    pub letter: Option<char>,
}

/// A fish swimming behind the card from `from` to `to` (mm) in `swim_ms`,
/// then out of sight for `rest_ms`, `start_ms` into its round at the
/// clock's 0, drawn at `scale_pct`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitleFish {
    pub from: [i32; 3],
    pub to: [i32; 3],
    pub swim_ms: i32,
    pub rest_ms: i32,
    #[serde(default)]
    pub start_ms: i32,
    pub scale_pct: i32,
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
    /// how far from the camera the map's live things (what sways, what
    /// is see-through) are drawn
    pub live_mm: i32,
    /// how long a "-2" popup shows
    pub popup_ms: i32,
    pub overhead: OverheadLook,
}

/// The overhead view in the bottom-left corner (the top-left on touch).
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
    /// the ghost's ring round a square's foot
    pub board: MarkLie,
    /// a word's outline at the board
    pub outline: OutlineLook,
    /// the ring round a tile a free claw could grab
    pub grab: GrabLook,
    /// the outer corners' radius
    pub radius_mm: i32,
    /// how far each mark sits above the one before
    pub step_mm: i32,
    /// about how long a dash is
    pub dash_mm: i32,
    /// the ghost tile's ring, 0-255
    pub ghost: [i32; 3],
}

/// A word's outline (`crate::marks`), mm from a square's middle and up:
/// a strip on the board from the wall out to `foot_out_mm` at `foot_y_mm`,
/// the wall up the tile's side to `top_y_mm`, and the rim on the tile's top
/// from `rim_in_mm` to the wall. A dashed outline has `dashes` dashes to a
/// square's side, each `dash_pct` percent of its share.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutlineLook {
    pub foot_y_mm: i32,
    pub top_y_mm: i32,
    pub rim_in_mm: i32,
    pub wall_mm: i32,
    pub foot_out_mm: i32,
    pub dashes: i32,
    pub dash_pct: i32,
}

/// The ring round a tile a free claw could grab (`marks::draw_grab`), mm
/// in the tile's frame: its inner edge from the tile's middle, its width,
/// how high its top stands over the tile's bottom face (its collar hangs to
/// the bottom), and how solid it is.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrabLook {
    pub inner_mm: i32,
    pub width_mm: i32,
    pub top_mm: i32,
    pub alpha_pct: i32,
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
        let cb = &look.colourblind;
        named("colourblind player colour", &cb.players, &PLAYER_COLOURS)?;
        named("colourblind ink", &cb.ink, &INKS)?;
        if !cb.board.values().all(colour_ok)
            || !colour_ok(&cb.symbol_ink)
            || !colour_ok(&cb.symbol_rim)
            || !(100..=300).contains(&cb.thick_pct)
            || !(1..=1000).contains(&cb.symbol_mm)
            || !(0..=2000).contains(&cb.symbol_up_mm)
        {
            return Err("The colourblind look is out of range.".into());
        }
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
        let o = &m.outline;
        let outline_ok = 0 < o.rim_in_mm
            && o.rim_in_mm < o.wall_mm
            && o.wall_mm < o.foot_out_mm
            && o.foot_out_mm <= 2 * 105 - o.rim_in_mm
            && o.foot_y_mm < o.top_y_mm
            && (1..=8).contains(&o.dashes)
            && (10..=90).contains(&o.dash_pct);
        let g = &m.grab;
        let grab_ok = g.inner_mm > 0 && g.width_mm > 0 && g.top_mm > 0 && (1..=100).contains(&g.alpha_pct);
        if !lie_ok(&m.board)
            || !outline_ok
            || !grab_ok
            || m.dash_mm <= 0
            || m.step_mm < 0
            || !colour_ok(&m.ghost)
        {
            return Err("The word marks' sizes are out of range.".into());
        }
        for c in &look.title.crabs {
            if !PLAYER_COLOURS.contains(&c.colour.as_str())
                || c.speed_mm_per_s <= 0
                || c.rest_ms.iter().any(|r| *r < 0)
                || c.from == c.to
                || c.letter.is_some_and(|l| crate::letters::value(l).is_none())
            {
                return Err("A title crab's colour, path, speed, rests or letter is out of range.".into());
            }
        }
        if look
            .title
            .fish
            .iter()
            .any(|f| f.swim_ms <= 0 || f.rest_ms < 0 || f.scale_pct <= 0)
        {
            return Err("A title fish's times or size are out of range.".into());
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
            || !(0..=p.far_mm).contains(&p.live_mm)
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
        let t = &look.touch;
        if t.short_px <= 0
            || !(100..=300).contains(&t.scale_max_pct)
            || [t.claw_px, t.jump_px, t.small_px, t.stick_px]
                .iter()
                .any(|v| !(20..=300).contains(v))
            || !(0..=100).contains(&t.margin_px)
            || !(0..=100).contains(&t.gap_px)
            || !(5..=95).contains(&t.dead_pct)
            || !(10..=1000).contains(&t.look_pct)
        {
            return Err("The touch controls' sizes are out of range.".into());
        }
        look.crab.check()?;
        let t = &look.tally;
        if [t.first_ms, t.line_ms, t.gap_ms, t.winner_ms]
            .iter()
            .any(|ms| !(0..=10_000).contains(ms))
        {
            return Err("The tally's times are outside 0 to 10000 ms.".into());
        }
        let t = &look.title;
        if !(0..=30).contains(&t.tilt_deg)
            || !(0..=100).contains(&t.bob_mm)
            || !(0..=15).contains(&t.rock_deg)
            || !(0..=1000).contains(&t.rise_mm)
            || !(0..=100).contains(&t.sway_mm)
            || !(0..=10).contains(&t.drift_deg)
            || [t.bob_ms, t.rise_ms, t.drift_ms]
                .iter()
                .any(|ms| !(500..=120_000).contains(ms))
        {
            return Err("The title card's camera or motion is out of range.".into());
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
        let e = with(|v| v["marks"]["board"]["width_mm"] = serde_json::json!(60));
        assert_eq!(e, Err("The word marks' sizes are out of range.".into()));
        // an outline's rim with no width inside its wall, or a dashed one
        // with no gaps
        let e = with(|v| v["marks"]["outline"]["rim_in_mm"] = serde_json::json!(0));
        assert_eq!(e, Err("The word marks' sizes are out of range.".into()));
        let e = with(|v| v["marks"]["outline"]["dash_pct"] = serde_json::json!(100));
        assert_eq!(e, Err("The word marks' sizes are out of range.".into()));
        // a grab ring that cannot be seen
        let e = with(|v| v["marks"]["grab"]["alpha_pct"] = serde_json::json!(0));
        assert_eq!(e, Err("The word marks' sizes are out of range.".into()));
        // colourblind outlines thinner than the usual ones, or a seat
        // without its colourblind colour
        let e = with(|v| v["colourblind"]["thick_pct"] = serde_json::json!(50));
        assert_eq!(e, Err("The colourblind look is out of range.".into()));
        let e = with(|v| {
            v["colourblind"]["players"]
                .as_object_mut()
                .unwrap()
                .remove("blue");
        });
        assert_eq!(
            e,
            Err("The look file has no colourblind player colour \"blue\".".into())
        );
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

    #[test]
    fn the_title_cards_camera_and_motion_are_checked() {
        let refused = Err("The title card's camera or motion is out of range.".to_string());
        assert_eq!(with(|v| v["title"]["tilt_deg"] = serde_json::json!(31)), refused);
        assert_eq!(with(|v| v["title"]["bob_mm"] = serde_json::json!(-1)), refused);
        // a cycle of no time would divide by nothing
        assert_eq!(with(|v| v["title"]["rise_ms"] = serde_json::json!(0)), refused);
        assert_eq!(with(|v| v["title"]["drift_ms"] = serde_json::json!(499)), refused);
        assert!(with(|v| v["title"]["drift_ms"] = serde_json::json!(500)).is_ok());
    }

    #[test]
    fn the_title_crabs_and_fish_are_checked() {
        let refused = Err("A title crab's colour, path, speed, rests or letter is out of range.".to_string());
        assert_eq!(
            with(|v| v["title"]["crabs"][0]["colour"] = serde_json::json!("green")),
            refused
        );
        // a crab that never moves would rest for ever
        assert_eq!(
            with(|v| v["title"]["crabs"][0]["speed_mm_per_s"] = serde_json::json!(0)),
            refused
        );
        assert_eq!(
            with(|v| v["title"]["crabs"][0]["letter"] = serde_json::json!("?")),
            refused
        );
        assert!(with(|v| v["title"]["crabs"][0]["colour"] = serde_json::json!("purple")).is_ok());
        let fish = Err("A title fish's times or size are out of range.".to_string());
        assert_eq!(
            with(|v| v["title"]["fish"][0]["swim_ms"] = serde_json::json!(0)),
            fish
        );
    }
}
