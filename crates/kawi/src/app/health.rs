use anyhow::Result;
use std::path::PathBuf;

use super::App;
use crate::doctor::{health_text as doctor_health_text, HealthRequest};
use crate::state::StatePaths;

const HEALTH_TOPIC: &str = "health";
const HEALTH_CHECKS: &[(&str, &str)] = &[
    ("config", "config"),
    ("debug", "debug"),
    ("git", "git"),
    ("keymaps", "keymaps"),
    ("languages", "languages"),
    ("lsp", "languages"),
    ("paths", "paths"),
    ("plugin-keybindings", "plugin_keybindings"),
    ("plugins", "plugins"),
    ("session", "session"),
    ("state", "state"),
    ("tasks", "tasks"),
];

impl App {
    pub(super) fn handle_health(&mut self, topic: Option<&str>) -> Result<bool> {
        let filter = match topic {
            None => None,
            Some(topic) => match health_check_id(topic) {
                Some(id) => Some(id),
                None => {
                    self.show_message(format!("E149: Sorry, no health check for {topic}"));
                    return Ok(false);
                }
            },
        };
        let paths = StatePaths::resolve();
        let text = self.health_report(filter, paths.as_ref());
        self.show_health_text(text)?;
        Ok(false)
    }

    fn health_report(&self, filter: Option<&str>, paths: Option<&StatePaths>) -> String {
        let target = self.health_target();
        let workspace = self.project_root();
        doctor_health_text(HealthRequest {
            target: &target,
            workspace: &workspace,
            paths,
            filter,
        })
    }

    fn health_target(&self) -> PathBuf {
        let cwd = std::env::current_dir().unwrap_or_default();
        if self.editor.has_file() {
            kawi_editor::paths::absolute(self.editor.filename(), &cwd)
        } else {
            cwd
        }
    }

    fn show_health_text(&mut self, text: String) -> Result<()> {
        let lines = text.lines().map(str::to_string).collect();
        self.show_help_text(HEALTH_TOPIC, lines)
    }
}

fn health_check_id(topic: &str) -> Option<&'static str> {
    HEALTH_CHECKS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(topic))
        .map(|(_, id)| *id)
}

#[cfg(test)]
mod tests;
