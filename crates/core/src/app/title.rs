//! The Title: the wordmark and its entries. The entries are one list,
//! [`ENTRIES`], so another screen joins the Title as one more entry: add
//! its action here (its label is the string of the same key) and its arm in
//! [`App::title_entry`].

use super::*;

/// The Title's entries, top to bottom: each is an action name and the key
/// of its label.
pub const ENTRIES: &[&str] = &[
    "menu.local",
    "menu.online",
    "menu.how",
    "menu.settings",
    "menu.notes",
];

impl App {
    /// One of the Title's entries was pressed.
    pub(super) fn title_entry(&mut self, name: &str) {
        match name {
            "menu.local" => self.go(Screen::Local),
            "menu.online" => self.go(Screen::Online),
            "menu.how" => self.go(Screen::How),
            "menu.settings" => {
                self.settings_return = self.screen == Screen::Play;
                self.go(Screen::Settings);
            }
            "menu.notes" => self.go(Screen::Notes),
            _ => {}
        }
    }

    /// The wordmark (a stand-in until the title card is built), then the
    /// entries.
    pub(super) fn title_panels(&self) -> Vec<Panel> {
        vec![
            Panel::new("title", self.t("title"), Vec::new()),
            Panel::new(
                "menu",
                String::new(),
                ENTRIES.iter().map(|id| self.button(id)).collect(),
            ),
        ]
    }
}
