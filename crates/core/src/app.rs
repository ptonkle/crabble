//! The app: which screen is up, the reader's settings, the game being
//! played, and what the page shows (`ui`). Everything changes through
//! `act(name, value)` and the page's clock (`advance`), so no state lives in
//! the page, and nothing here reads a clock or draws a random number: the
//! seed and the client id come from the page as flags.
//!
//! The screens follow Ape Race's menus: the Title, then a Local game (its
//! setup, then Play) or an Online game (Host or Join, the Lobby, then Play);
//! Settings; What's New. Play is the game (`sim::Match`, `app/play.rs`)
//! and its 3D view (`app/view3d.rs`). The Title's entries are one list,
//! `title::ENTRIES`.

use serde::Serialize;

use crate::patch_notes::{self, Entry};
use crate::strings::{self, Strings};
use crate::view::{Item, Panel};

mod fast;
mod how;
mod local;
mod notes;
pub mod online_play;
pub mod play;
mod settings;
#[cfg(test)]
mod tests;
pub mod title;
pub mod view3d;

/// The game's version: major.minor.patch. The newest entry of
/// `data/patch-notes.json` must be this one (`tests/data_files.rs`);
/// `scripts/bump.sh` moves both.
pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Screen {
    Title,
    /// the Local game's setup: how many AI crabs (`app/local.rs`)
    Local,
    /// Host, or Join by code (`app/online_play.rs`)
    Online,
    /// an online game's seats before it starts
    Lobby,
    /// a game, its pause menu and its end (`app/play.rs`)
    Play,
    /// How to play: a few lines and the controls (`app/how.rs`)
    How,
    Settings,
    /// What's New: each version's notes (`patch_notes.rs`)
    Notes,
    /// each model and scene (`crate::gallery`, drawn by `web/gallery.js`;
    /// its Back sends `menu.back`)
    Gallery,
}

/// The volume slider's range and step, in percent.
pub const VOLUME_MAX: u32 = 100;
pub const VOLUME_STEP: u32 = 5;
/// The mouse sensitivity slider's range and step, in percent of the tuning
/// file's turn (`look.turn_per_px`).
pub const MOUSE_MIN: u32 = 25;
pub const MOUSE_MAX: u32 = 300;
pub const MOUSE_STEP: u32 = 5;

/// The reader's settings, kept between visits by `web/settings-store.js`
/// (BUILDER-SPEC, the D17 change; SECOND-ORDER R1.23; `app/settings.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// 0 to `VOLUME_MAX`, a multiple of `VOLUME_STEP`; no sound plays yet
    pub volume: u32,
    /// `MOUSE_MIN` to `MOUSE_MAX`
    pub mouse_pct: u32,
    pub invert_y: bool,
    /// What's New lists the minor versions too
    pub notes_minor: bool,
    /// the player's name online; empty for the default
    pub name: String,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            volume: 80,
            mouse_pct: 100,
            invert_y: false,
            notes_minor: false,
            name: String::new(),
        }
    }
}

/// Everything the page draws.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ui {
    pub screen: Screen,
    /// the document title
    pub title: String,
    /// shown in the corner, as "v0.1.0"
    pub version: String,
    /// whether clicking the version opens What's New
    pub version_opens: bool,
    /// for the sounds to come, 0 to `VOLUME_MAX`
    pub volume: u32,
    /// the page draws the game's 3D view (`frame`) behind the panels
    pub scene: bool,
    /// the page shows the crosshair in the middle of the view
    pub crosshair: bool,
    /// the page keeps keys and the mouse from the browser and sends them
    /// here (a click locks the pointer)
    pub capture: bool,
    /// the page asks for the view model each frame (a game's clock moves)
    pub live: bool,
    /// for the browser checks: the game's phase (`countdown`, `running`,
    /// `ended`, or empty), and whether the local crab has left its start
    pub phase: String,
    pub moved: bool,
    pub panels: Vec<Panel>,
    /// the settings as the page keeps them between visits (`settings.load`
    /// takes them back)
    pub settings: String,
}

pub struct App {
    strings: Strings,
    notes: Vec<Entry>,
    pub settings: Settings,
    pub screen: Screen,
    /// Settings was opened from the pause menu: Back returns to Play
    pub settings_return: bool,
    /// the sounds the page plays next, as the core names them
    /// (`net_pump` hands them over)
    pub cues: Vec<play::Cue>,
    /// how many sounds have been named, for each one's seed
    pub cue_count: u64,
    /// the game's numbers (`data.tuning`)
    pub tuning: Option<crate::tuning::Tuning>,
    /// the 3D view's files and frames (`data.models`, `data.glyphs`,
    /// `data.look`, `data.map`)
    pub view: view3d::View3d,
    /// the Local game's setup: how many AI crabs
    pub local_ai: usize,
    /// the game being played
    pub play: Option<play::Play>,
    /// the page's clock, in microseconds (`advance`)
    pub clock: i64,
    /// the drawing buffer's size, in pixels (`resize`)
    pub size: [i32; 2],
    /// the page's seed (`flag.seed`), mixed into each game's
    pub seed: u64,
    /// the page's `?fast`: the browser checks' fast mode (`app/fast.rs`)
    pub fast: bool,
    /// the page's `?short`: a short game, for the browser checks to reach
    /// the end (`app/fast.rs`)
    pub short: Option<i64>,
    /// the page's `?spot`: where the local crab starts (`app/fast.rs`)
    pub spot: Option<(i32, i32, i32)>,
    /// online play: the session, its codes and the transport's requests
    pub online: online_play::OnlinePlay,
    /// whether the page can draw 3D (`flag.webgl`), once it has said
    pub webgl: Option<bool>,
}

impl App {
    /// Checks the strings file and the patch notes, and opens on the Title.
    pub fn new(strings_json: &str, notes_json: &str) -> Result<App, String> {
        let strings = strings::parse(strings_json).map_err(|e| e.message())?;
        let notes = patch_notes::parse(notes_json)?;
        Ok(App {
            strings,
            notes,
            settings: Settings::default(),
            screen: Screen::Title,
            settings_return: false,
            tuning: None,
            view: view3d::View3d::default(),
            local_ai: 1,
            cues: Vec::new(),
            cue_count: 0,
            play: None,
            clock: 0,
            size: [1, 1],
            seed: 0,
            fast: false,
            short: None,
            spot: None,
            online: online_play::OnlinePlay::default(),
            webgl: None,
        })
    }

    pub(crate) fn t(&self, key: &str) -> String {
        self.strings.t(key)
    }

    pub(crate) fn button(&self, id: &str) -> Item {
        Item::button(id, self.t(id))
    }

    pub(crate) fn now_ms(&self) -> i64 {
        self.clock / 1000
    }

    // ---- screens ----

    pub(crate) fn go(&mut self, screen: Screen) {
        self.screen = screen;
    }

    /// Back, and Escape off a sub-screen: to the Title, or from Settings
    /// opened in a game back to the game.
    fn back(&mut self) {
        if self.screen == Screen::Settings && self.settings_return && self.play.is_some() {
            self.settings_return = false;
            self.go(Screen::Play);
        } else {
            self.settings_return = false;
            self.go(Screen::Title);
        }
    }

    fn escape(&mut self) {
        match self.screen {
            Screen::Play => self.toggle_pause(),
            Screen::Settings | Screen::Notes | Screen::How | Screen::Local | Screen::Online => self.back(),
            // leaving a lobby is its own button, not a stray Escape; the
            // Gallery has its own Back
            Screen::Title | Screen::Lobby | Screen::Gallery => {}
        }
    }

    /// The version in the corner opens What's New from the menus, not
    /// from Play.
    pub fn version_opens_notes(&self) -> bool {
        self.screen != Screen::Play
    }

    /// The one door for buttons, settings, keys and screens. `name` is a
    /// dotted action name, `value` the control's new value.
    pub fn act(&mut self, name: &str, value: &str) -> Result<(), String> {
        if self.play_input(name, value) {
            return Ok(());
        }
        match name {
            "key" if value == "Escape" => self.escape(),
            "key" | "key.up" | "keys.release" | "mouse.move" | "mouse.down" | "pointer.released" => {}
            _ if title::ENTRIES.contains(&name) => self.title_entry(name),
            "menu.back" => self.back(),
            "version" if self.version_opens_notes() => self.go(Screen::Notes),
            "version" => {}
            "notes.minor" => self.settings.notes_minor = value == "on",
            "flag.fast" | "flag.short" | "flag.seed" | "flag.spot" => self.test_flag(name, value)?,
            // whether the page could make its 3D renderer (`app/title.rs`)
            "flag.webgl" => self.webgl = Some(value == "on"),
            "data.tuning" => self.tuning = Some(crate::tuning::parse(value)?),
            "data.models" | "data.glyphs" | "data.look" | "data.map" | "data.title" => {
                self.view.load(name, value)?
            }
            // checked here so a cue the core names is one the file holds
            "data.sounds" => play::check_cues(value)?,
            _ if name.starts_with("local.") => self.local_act(name, value)?,
            _ if name.starts_with("settings.") => self.settings_act(name, value)?,
            _ if name.starts_with("menu.") || name.starts_with("end.") => self.play_act(name)?,
            _ if name.starts_with("online.") || name.starts_with("flag.") => self.online_act(name, value)?,
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    /// The page's clock, in microseconds. It only goes forward; the game
    /// being played moves on with it.
    pub fn advance(&mut self, now: i64) {
        let before = self.now_ms();
        self.clock = self.clock.max(now);
        let ms = self.now_ms() - before;
        self.play_advance(ms);
    }

    /// The canvas's size in CSS pixels and the device's pixel ratio in
    /// hundredths; returns the drawing buffer's size.
    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_pct: i32) -> [i32; 2] {
        let dpr = dpr_pct.clamp(100, 300);
        self.size = [
            (css_w.max(1) as i64 * dpr as i64 / 100) as i32,
            (css_h.max(1) as i64 * dpr as i64 / 100) as i32,
        ];
        self.size
    }

    pub fn ui(&self) -> Ui {
        let panels = match self.screen {
            Screen::Title => self.title_panels(),
            Screen::Local => self.local_panels(),
            Screen::Online => self.online_panels(),
            Screen::Lobby => self.lobby_panels(),
            Screen::Play => self.play_panels(),
            Screen::Settings => self.settings_panels(),
            Screen::Notes => self.notes_panels(),
            Screen::How => self.how_panels(),
            Screen::Gallery => Vec::new(),
        };
        let playing = self.screen == Screen::Play && self.play.is_some();
        Ui {
            screen: self.screen,
            title: self.t("title"),
            version: format!("v{VERSION}"),
            version_opens: self.version_opens_notes(),
            volume: self.settings.volume,
            scene: playing || self.title_card(),
            crosshair: playing
                && self
                    .play
                    .as_ref()
                    .is_some_and(|p| p.captures() && !p.keys.overhead),
            capture: playing && self.play.as_ref().is_some_and(|p| p.captures()),
            live: playing || matches!(self.screen, Screen::Lobby),
            phase: if playing { self.phase_key() } else { String::new() },
            moved: self.play.as_ref().is_some_and(|p| p.local_moved()),
            panels,
            settings: self.stored_settings(),
        }
    }
}
