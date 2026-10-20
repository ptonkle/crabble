//! The game's numbers, from `data/tuning.json`: the game's length, its
//! tick and countdown, the board's size, how a crab walks, jumps and
//! reaches, the camera that is also the aim, the tiles' tumbling and the
//! idle penalty. The page fetches the file and hands it over
//! (`data.tuning`); the core checks every value before a game can start.
//! A number here is a rule; a number for drawing only lives in
//! `data/look.json`.

use serde::Deserialize;

use crate::gfx::fx::ONE;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    /// a game's length on the clock
    pub minutes: u32,
    /// the 3-2-1 before it
    pub countdown_seconds: u32,
    /// one step of the simulation (`sim::Match::step`)
    pub tick_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Board {
    /// squares on a side
    pub size: u32,
    /// one square's side, in millimetres
    pub square_mm: i32,
    /// the height of the board's top above the sand, where tiles sit
    pub top_mm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crab {
    pub speed_mm_per_s: i32,
    /// how high a jump rises, and how long it is in the air
    pub jump_apex_mm: i32,
    pub jump_air_ms: i32,
    /// the fastest a crab falls
    pub fall_max_mm_per_s: i32,
    /// the highest tile edge it walks up without a jump, and the highest
    /// edge of the drawn ground it scrambles up (the board's rim)
    pub step_mm: i32,
    pub climb_mm: i32,
    /// how far round its middle it stands on the ground (its feet)
    pub foot_mm: i32,
    /// how close two crabs come before they push apart, and how much of
    /// the overlap goes each tick
    pub radius_mm: i32,
    pub push_pct: i32,
    /// how far ahead of its middle its claw tips are
    pub claw_tip_mm: i32,
    /// how far off its corner of the board a crab starts, along the diagonal
    pub start_off_corner_mm: i32,
    /// how far past the board's edge a crab may walk
    pub roam_mm: i32,
    /// how far beyond its claw tips a crab can grab or place a tile: about
    /// one board square (BRIEF Q3; SECOND-ORDER R1.186)
    pub reach_mm: i32,
}

/// The third-person camera, whose centre ray is the aim.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub distance_mm: i32,
    pub pivot_up_mm: i32,
    pub clear_mm: i32,
}

/// The camera's turn, in angle units (65536 a full turn).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    /// per pixel of mouse movement at 100% sensitivity
    pub turn_per_px: i32,
    pub pitch_start: i32,
    pub pitch_min: i32,
    pub pitch_max: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tuning {
    pub game: Game,
    pub board: Board,
    pub crab: Crab,
    pub camera: Camera,
    pub look: Look,
    /// the AI crabs' numbers (`crate::ai`)
    pub ai: crate::ai::Numbers,
}

impl Tuning {
    /// A game's length, in milliseconds.
    pub fn game_ms(&self) -> i64 {
        self.game.minutes as i64 * 60_000
    }

    /// The board's side, in millimetres.
    pub fn board_mm(&self) -> i32 {
        self.board.size as i32 * self.board.square_mm
    }

    /// A jump's take-off speed and the pull down, in Q16 metres a second
    /// and a second squared, from its apex `h` and its time in the air
    /// `T`: it rises for T/2, so v = 4h/T and g = 8h/T².
    pub fn jump(&self) -> (i32, i32) {
        let h = self.crab.jump_apex_mm as i64 * ONE as i64 / 1000;
        let t = self.crab.jump_air_ms as i64;
        ((4 * h * 1000 / t) as i32, (8 * h * 1_000_000 / (t * t)) as i32)
    }
}

/// Parses and checks the tuning file. Keys starting with `_` are notes.
pub fn parse(json: &str) -> Result<Tuning, String> {
    let mut v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("The tuning file could not be read: {e}"))?;
    strip_notes(&mut v);
    let t: Tuning =
        serde_json::from_value(v).map_err(|e| format!("The tuning file could not be read: {e}"))?;
    let c = &t.crab;
    let checks: [(&str, i64, i64, i64); 24] = [
        ("game.minutes", t.game.minutes as i64, 1, 60),
        ("game.countdown_seconds", t.game.countdown_seconds as i64, 0, 10),
        ("game.tick_ms", t.game.tick_ms as i64, 5, 100),
        ("board.size", t.board.size as i64, 3, 31),
        ("board.square_mm", t.board.square_mm as i64, 50, 1000),
        ("board.top_mm", t.board.top_mm as i64, 0, 500),
        ("crab.speed_mm_per_s", c.speed_mm_per_s as i64, 1, 20_000),
        ("crab.jump_apex_mm", c.jump_apex_mm as i64, 1, 2000),
        ("crab.jump_air_ms", c.jump_air_ms as i64, 100, 5000),
        ("crab.fall_max_mm_per_s", c.fall_max_mm_per_s as i64, 1, 20_000),
        ("crab.step_mm", c.step_mm as i64, 0, 500),
        ("crab.climb_mm", c.climb_mm as i64, 0, 500),
        ("crab.foot_mm", c.foot_mm as i64, 0, 500),
        ("crab.radius_mm", c.radius_mm as i64, 1, 1000),
        ("crab.push_pct", c.push_pct as i64, 0, 100),
        ("crab.claw_tip_mm", c.claw_tip_mm as i64, 0, 1000),
        ("crab.start_off_corner_mm", c.start_off_corner_mm as i64, 0, 5000),
        ("crab.roam_mm", c.roam_mm as i64, 0, 20_000),
        ("crab.reach_mm", c.reach_mm as i64, 1, 2000),
        ("camera.distance_mm", t.camera.distance_mm as i64, 50, 5000),
        ("camera.pivot_up_mm", t.camera.pivot_up_mm as i64, 0, 2000),
        ("camera.clear_mm", t.camera.clear_mm as i64, 0, 500),
        ("look.turn_per_px", t.look.turn_per_px as i64, 1, 1000),
        ("look.pitch_start", t.look.pitch_start as i64, -16383, 16383),
    ];
    for (name, value, lo, hi) in checks {
        if value < lo || value > hi {
            return Err(format!(
                "The tuning file's {name} is {value}; it must be {lo} to {hi}."
            ));
        }
    }
    t.ai.check()?;
    let l = &t.look;
    if !(-16384 < l.pitch_min
        && l.pitch_min <= l.pitch_start
        && l.pitch_start <= l.pitch_max
        && l.pitch_max < 16384)
    {
        return Err(
            "The tuning file's look pitches must rise from pitch_min to pitch_max, within a quarter turn."
                .into(),
        );
    }
    Ok(t)
}

/// Removes every key starting with `_`, at any depth.
pub fn strip_notes(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(map) => {
            map.retain(|k, _| !k.starts_with('_'));
            for x in map.values_mut() {
                strip_notes(x);
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(strip_notes),
        _ => {}
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const SHIPPED: &str = include_str!("../../../data/tuning.json");

    #[test]
    fn the_shipped_tuning_is_a_four_minute_game_on_a_fifteen_square_board() {
        let t = parse(SHIPPED).unwrap();
        assert_eq!(t.game.minutes, 4);
        assert_eq!(t.game_ms(), 240_000);
        assert_eq!(t.board.size, 15);
        assert_eq!(t.board_mm(), 15 * 210);
    }

    #[test]
    fn the_shipped_crab_crosses_the_board_in_eight_seconds_and_jumps_over_a_tile() {
        // the person, R1.187: "8 seconds to cross the board, jump just high
        // enough to jump on top of tiles on the board"
        let t = parse(SHIPPED).unwrap();
        let ms = t.board_mm() as i64 * 1000 / t.crab.speed_mm_per_s as i64;
        assert!((7900..=8100).contains(&ms), "{ms} ms to cross");
        // a tile lies 45 mm high; the apex clears it with a little to spare
        assert!((50..=80).contains(&t.crab.jump_apex_mm));
        assert!(t.crab.step_mm < 45, "a lying tile needs a jump");
    }

    #[test]
    fn the_jump_rises_to_its_apex_in_half_its_air_time() {
        let mut t = parse(SHIPPED).unwrap();
        t.crab.jump_apex_mm = 100;
        t.crab.jump_air_ms = 1000;
        let (v, g) = t.jump();
        // 0.1 m in 0.5 s up: v = 0.4 m/s, g = 0.8 m/s²
        assert!((v - ONE * 4 / 10).abs() <= 4, "{v}");
        assert!((g - ONE * 8 / 10).abs() <= 8, "{g}");
    }

    #[test]
    fn a_value_out_of_range_or_an_unknown_key_is_refused_by_name() {
        let zero = SHIPPED.replace("\"minutes\": 4", "\"minutes\": 0");
        assert_eq!(
            parse(&zero).unwrap_err(),
            "The tuning file's game.minutes is 0; it must be 1 to 60."
        );
        let extra = SHIPPED.replace("\"minutes\": 4", "\"minutes\": 4, \"hours\": 1");
        assert!(parse(&extra).unwrap_err().contains("hours"));
    }
}
