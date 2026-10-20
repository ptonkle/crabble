//! The game's numbers, from `data/tuning.json`: the game's length and
//! countdown, the board's size, how fast a crab walks and how far the mouse
//! turns the camera. The page fetches the file and hands it over
//! (`data.tuning`); the core checks every value before a game can start.
//! A number here is a rule; a number for drawing only lives in
//! `data/standin.json` (`standin::look`).

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    /// a game's length on the clock
    pub minutes: u32,
    /// the 3-2-1 before it
    pub countdown_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Board {
    /// squares on a side
    pub size: u32,
    /// one square's side, in millimetres
    pub square_mm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Crab {
    pub speed_mm_per_s: i32,
    /// how far off its corner of the board a crab starts, along the diagonal
    pub start_off_corner_mm: i32,
    /// how far past the board's edge a crab may walk
    pub roam_mm: i32,
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

/// The stand-in AI crabs' wander (`standin`), until the real AI is built.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wander {
    /// how close to its target point an AI crab turns to a new one
    pub arrive_mm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tuning {
    pub game: Game,
    pub board: Board,
    pub crab: Crab,
    pub look: Look,
    pub wander: Wander,
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
}

/// Parses and checks the tuning file. Keys starting with `_` are notes.
pub fn parse(json: &str) -> Result<Tuning, String> {
    let mut v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("The tuning file could not be read: {e}"))?;
    strip_notes(&mut v);
    let t: Tuning =
        serde_json::from_value(v).map_err(|e| format!("The tuning file could not be read: {e}"))?;
    let checks: [(&str, i64, i64, i64); 9] = [
        ("game.minutes", t.game.minutes as i64, 1, 60),
        ("game.countdown_seconds", t.game.countdown_seconds as i64, 0, 10),
        ("board.size", t.board.size as i64, 3, 31),
        ("board.square_mm", t.board.square_mm as i64, 50, 1000),
        ("crab.speed_mm_per_s", t.crab.speed_mm_per_s as i64, 1, 20_000),
        (
            "crab.start_off_corner_mm",
            t.crab.start_off_corner_mm as i64,
            0,
            5000,
        ),
        ("crab.roam_mm", t.crab.roam_mm as i64, 0, 50_000),
        ("look.turn_per_px", t.look.turn_per_px as i64, 1, 1000),
        ("wander.arrive_mm", t.wander.arrive_mm as i64, 1, 5000),
    ];
    for (name, value, lo, hi) in checks {
        if value < lo || value > hi {
            return Err(format!(
                "The tuning file's {name} is {value}; it must be {lo} to {hi}."
            ));
        }
    }
    let l = &t.look;
    if !(0 <= l.pitch_min
        && l.pitch_min <= l.pitch_start
        && l.pitch_start <= l.pitch_max
        && l.pitch_max < 16384)
    {
        return Err(
            "The tuning file's look pitches must rise from pitch_min to pitch_max, under a quarter turn."
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
