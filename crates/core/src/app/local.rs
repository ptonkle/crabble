//! The Local game's setup: how many AI crabs (1 to 3), then Start. One AI
//! level only (the person's decision, 2026-10-19), so there is no
//! difficulty to choose.

use super::*;
use crate::view::Choice;

/// The most AI crabs in a Local game: three, so four crabs in all.
pub const LOCAL_AI_MAX: usize = crate::online::MAX_AI;

impl App {
    pub(super) fn local_act(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "local.ai" => {
                let n: usize = value
                    .parse()
                    .map_err(|_| format!("The AI crab count \"{value}\" is not a whole number."))?;
                if !(1..=LOCAL_AI_MAX).contains(&n) {
                    return Err(format!("A Local game has 1 to {LOCAL_AI_MAX} AI crabs, not {n}."));
                }
                self.local_ai = n;
            }
            "local.start" => self.start_local()?,
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    pub(super) fn ai_choices(&self, from: usize, to: usize) -> Vec<Choice> {
        (from..=to)
            .map(|n| Choice {
                value: n.to_string(),
                text: n.to_string(),
                disabled: false,
            })
            .collect()
    }

    pub(super) fn local_panels(&self) -> Vec<Panel> {
        vec![Panel::new(
            "local",
            self.t("menu.local"),
            vec![
                Item::Select {
                    id: "local.ai".into(),
                    label: self.t("setup.ai"),
                    value: self.local_ai.to_string(),
                    options: self.ai_choices(1, LOCAL_AI_MAX),
                },
                self.button("local.start"),
                self.button("menu.back"),
            ],
        )]
    }
}
