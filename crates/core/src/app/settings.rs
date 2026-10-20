//! Settings: the volume, the mouse's sensitivity and whether it tilts the
//! camera the other way, kept between visits with the online name.
//!
//! The person chose to keep settings between visits (CR-Q2, "yes";
//! SECOND-ORDER R1.23; BUILDER-SPEC, the D17 change), as Honey Hole does.
//! The core decides what is kept and checks what comes back:
//! [`App::stored_settings`] writes the settings as one JSON object
//! (`Ui::settings`), which `web/settings-store.js` stores as it is under one
//! key; on the next visit the page hands the text back through
//! `act("settings.load", text)`. Every value goes through the setter its
//! control uses, and anything unreadable or off its slider is refused whole:
//! a bad save is treated as no save.

use serde::{Deserialize, Serialize};

use super::*;

/// What is kept between visits. A field missing from an older save keeps
/// its default; a field this version does not know is ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Kept {
    #[serde(default = "default_volume")]
    volume: u32,
    #[serde(default = "default_mouse")]
    mouse_pct: u32,
    #[serde(default)]
    invert_y: bool,
    #[serde(default)]
    notes_minor: bool,
    #[serde(default)]
    name: String,
    #[serde(default)]
    colourblind: bool,
}

fn default_volume() -> u32 {
    Settings::default().volume
}

fn default_mouse() -> u32 {
    Settings::default().mouse_pct
}

impl App {
    /// The settings as the page keeps them between visits.
    pub fn stored_settings(&self) -> String {
        let s = &self.settings;
        let kept = Kept {
            volume: s.volume,
            mouse_pct: s.mouse_pct,
            invert_y: s.invert_y,
            notes_minor: s.notes_minor,
            name: s.name.clone(),
            colourblind: s.colourblind,
        };
        serde_json::to_string(&kept).unwrap_or_default()
    }

    /// Settings kept from an earlier visit (`settings.load`): every value
    /// on its control's slider, all or nothing.
    fn load_settings(&mut self, text: &str) -> Result<(), String> {
        let kept: Kept =
            serde_json::from_str(text).map_err(|e| format!("The kept settings could not be read: {e}"))?;
        if kept.volume > VOLUME_MAX || !kept.volume.is_multiple_of(VOLUME_STEP) {
            return Err(format!("The kept volume {} is not on the slider.", kept.volume));
        }
        if !(MOUSE_MIN..=MOUSE_MAX).contains(&kept.mouse_pct) || !kept.mouse_pct.is_multiple_of(MOUSE_STEP) {
            return Err(format!(
                "The kept mouse sensitivity {} is not on the slider.",
                kept.mouse_pct
            ));
        }
        self.settings.volume = kept.volume;
        self.settings.mouse_pct = kept.mouse_pct;
        self.settings.invert_y = kept.invert_y;
        self.settings.notes_minor = kept.notes_minor;
        self.settings.name = clean_kept_name(&kept.name);
        self.settings.colourblind = kept.colourblind;
        Ok(())
    }

    /// The Settings screen's controls (`settings.*`).
    pub(super) fn settings_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        let number = || -> Result<u32, String> {
            value
                .parse()
                .map_err(|_| format!("The setting \"{value}\" is not a whole number."))
        };
        match name {
            // each slider's own steps, whatever the page sends
            "settings.volume" => self.settings.volume = number()?.min(VOLUME_MAX) / VOLUME_STEP * VOLUME_STEP,
            "settings.mouse" => {
                self.settings.mouse_pct = number()?.clamp(MOUSE_MIN, MOUSE_MAX) / MOUSE_STEP * MOUSE_STEP
            }
            "settings.invert_y" => self.settings.invert_y = value == "on",
            "settings.colourblind" => self.settings.colourblind = value == "on",
            "settings.load" => self.load_settings(value)?,
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    pub(super) fn settings_panels(&self) -> Vec<Panel> {
        let s = &self.settings;
        vec![Panel::new(
            "settings",
            self.t("settings.heading"),
            vec![
                Item::Slider {
                    id: "settings.volume".into(),
                    label: self.t("settings.volume"),
                    min: 0,
                    max: VOLUME_MAX,
                    step: VOLUME_STEP,
                    value: s.volume,
                    text: format!("{}%", s.volume),
                },
                Item::Slider {
                    id: "settings.mouse".into(),
                    label: self.t("settings.mouse"),
                    min: MOUSE_MIN,
                    max: MOUSE_MAX,
                    step: MOUSE_STEP,
                    value: s.mouse_pct,
                    text: format!("{}%", s.mouse_pct),
                },
                Item::Toggle {
                    id: "settings.invert_y".into(),
                    label: self.t("settings.invert_y"),
                    value: s.invert_y,
                },
                Item::Toggle {
                    id: "settings.colourblind".into(),
                    label: self.t("settings.colourblind"),
                    value: s.colourblind,
                },
                self.button("menu.back"),
            ],
        )]
    }
}

/// A kept name, cleaned as the name box cleans a typed one.
fn clean_kept_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_control())
        .take(crate::online::MAX_NAME_CHARS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        crate::app::tests::app()
    }

    #[test]
    fn kept_settings_come_back_on_the_next_visit() {
        let mut a = app();
        a.act("settings.volume", "40").unwrap();
        a.act("settings.mouse", "150").unwrap();
        a.act("settings.invert_y", "on").unwrap();
        a.act("settings.colourblind", "on").unwrap();
        a.act("online.name", "Pinchy").unwrap();
        let kept = a.ui().settings;
        let mut next = app();
        next.act("settings.load", &kept).unwrap();
        assert_eq!(next.settings.volume, 40);
        assert_eq!(next.settings.mouse_pct, 150);
        assert!(next.settings.invert_y);
        assert!(next.settings.colourblind);
        assert!(next.ui().colourblind, "the page is told");
        assert_eq!(next.settings.name, "Pinchy");
    }

    #[test]
    fn a_bad_save_is_refused_whole_and_changes_nothing() {
        let mut a = app();
        let before = a.settings.clone();
        assert!(a.act("settings.load", "{not json").is_err());
        // 42 is not on the volume slider's steps of 5
        assert!(a
            .act("settings.load", r#"{"volume": 42, "invert_y": true}"#)
            .is_err());
        assert!(a.act("settings.load", r#"{"mouse_pct": 1000}"#).is_err());
        assert_eq!(a.settings, before);
        // an older save without some fields keeps their defaults
        a.act("settings.load", r#"{"invert_y": true}"#).unwrap();
        assert_eq!(a.settings.volume, 80);
        assert!(a.settings.invert_y);
    }
}
