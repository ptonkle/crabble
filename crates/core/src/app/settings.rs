//! Settings: the volume, the mouse's sensitivity and whether it tilts the
//! camera the other way. They last for this visit (SECOND-ORDER R1.23).

use super::*;

impl App {
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
                self.button("menu.back"),
            ],
        )]
    }
}
