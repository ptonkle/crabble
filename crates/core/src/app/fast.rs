//! The page's test flags, never set for players:
//!
//! - **`?fast`** (`flag.fast`): skips the waits that only make a person
//!   wait, and leaves every rule's timing alone. Today that is the 3-2-1
//!   countdown before a game (`standin::Setup::fast`); a later wait (the
//!   tally's reveal, say) reads `App::fast` too. The browser checks open the
//!   page with it (`tests/browser/interactions.py`, `online_game.py`).
//! - **`?short`** (`flag.short`): a game of [`SHORT_GAME_MS`] instead of
//!   `game.minutes`, so the browser check reaches the end screen.
//! - **`flag.seed`**: the page's seed (from `crypto.getRandomValues`),
//!   mixed into each game's and each code's.

use super::App;

/// A `?short` game's length.
pub const SHORT_GAME_MS: i64 = 3000;

impl App {
    pub(super) fn test_flag(&mut self, name: &str, value: &str) -> Result<(), String> {
        let on = value != "off";
        match name {
            "flag.fast" => self.fast = on,
            "flag.short" => self.short = on,
            _ => {
                self.seed = value
                    .parse()
                    .map_err(|_| format!("The seed \"{value}\" is not a whole number."))?
            }
        }
        Ok(())
    }
}
