use crossterm::event::{KeyCode, KeyEvent};

use super::super::App;
use crate::plugin_manager::{PluginPrompt, PluginSpec, PromptAction};

impl App {
    pub(super) fn install_plugin(&mut self, rest: &str) {
        if rest.trim().is_empty() {
            self.show_message("Usage: plugin install <owner/repo[@ref]>".to_string());
            return;
        }
        let spec = match PluginSpec::parse(rest) {
            Ok(spec) => spec,
            Err(error) => {
                self.show_message(format!("Cannot install plugin: {error}"));
                return;
            }
        };
        match self.plugin_manager.prepare_install(&spec) {
            Ok(prepared) => self.show_plugin_prompt(PluginPrompt::install(prepared)),
            Err(error) => self.show_message(format!(
                "Cannot install plugin: {}",
                super::text::sanitize(&error)
            )),
        }
    }

    pub(super) fn update_plugin(&mut self, rest: &str) {
        let name = rest.trim();
        if name.is_empty() {
            self.show_message("Usage: plugin update <plugin>".to_string());
            return;
        }
        match self.plugin_manager.prepare_update(name) {
            Ok(update) if update.current => self.show_message(format!(
                "Plugin {} is already up to date",
                super::text::sanitize(name)
            )),
            Ok(update) if update.requires_confirmation => {
                self.show_plugin_prompt(PluginPrompt::update(update));
            }
            Ok(update) => self.apply_plugin_update(update),
            Err(error) => self.show_message(format!(
                "Cannot update plugin: {}",
                super::text::sanitize(&error)
            )),
        }
    }

    pub(super) fn remove_plugin(&mut self, rest: &str) {
        let name = rest.trim();
        if name.is_empty() {
            self.show_message("Usage: plugin remove <plugin>".to_string());
            return;
        }
        if self.reject_development_plugin(name) {
            return;
        }
        if !self.plugin_manager.is_managed(name) {
            self.show_message(format!(
                "Unknown installed plugin: {}",
                super::text::sanitize(name)
            ));
            return;
        }
        self.show_plugin_prompt(PluginPrompt::remove(name));
    }

    pub(super) fn set_plugin_enabled(&mut self, rest: &str, enabled: bool) {
        let name = rest.trim();
        if name.is_empty() {
            self.show_message(format!(
                "Usage: plugin {} <plugin>",
                if enabled { "enable" } else { "disable" }
            ));
            return;
        }
        if self.reject_development_plugin(name) {
            return;
        }
        match self.plugin_manager.set_enabled(name, enabled) {
            Ok(()) => {
                self.reload_plugins_after_change();
                self.show_message(format!(
                    "Plugin {}: {}",
                    if enabled { "enabled" } else { "disabled" },
                    super::text::sanitize(name)
                ));
            }
            Err(error) => self.show_message(format!(
                "Cannot change plugin state: {}",
                super::text::sanitize(&error)
            )),
        }
    }

    fn reject_development_plugin(&mut self, name: &str) -> bool {
        if !self.plugins.is_dev(name) {
            return false;
        }
        self.show_message(format!(
            "{} is a development plugin; remove it from {}",
            super::text::sanitize(name),
            self.plugin_manager.dev_dir().display()
        ));
        true
    }

    pub(crate) fn handle_plugin_prompt_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if let Some(prompt) = self.plugin_prompt.take() {
                    self.apply_plugin_prompt(prompt);
                }
            }
            KeyCode::Esc => {
                self.plugin_prompt = None;
                self.show_message("Plugin operation canceled".to_string());
            }
            _ => {}
        }
    }

    pub(crate) fn plugin_prompt_items(&self) -> Option<(&[String], usize)> {
        self.plugin_prompt
            .as_ref()
            .map(|prompt| (prompt.lines(), 0))
    }

    pub(crate) fn is_plugin_prompt_active(&self) -> bool {
        self.plugin_prompt.is_some()
    }

    fn show_plugin_prompt(&mut self, prompt: PluginPrompt) {
        self.show_message(prompt.message().to_string());
        self.plugin_prompt = Some(prompt);
    }

    fn apply_plugin_prompt(&mut self, prompt: PluginPrompt) {
        match prompt.action {
            PromptAction::Install(prepared) => match self.plugin_manager.apply_install(&prepared) {
                Ok(()) => {
                    self.reload_plugins_after_change();
                    self.show_message(format!("Installed plugin {}", prepared.name));
                }
                Err(error) => self.show_message(format!(
                    "Cannot install plugin: {}",
                    super::text::sanitize(&error)
                )),
            },
            PromptAction::Update(update) => self.apply_plugin_update(update),
            PromptAction::Remove(name) => match self.plugin_manager.remove(&name) {
                Ok(()) => {
                    self.reload_plugins_after_change();
                    self.show_message(format!("Removed plugin {}", super::text::sanitize(&name)));
                }
                Err(error) => self.show_message(format!(
                    "Cannot remove plugin: {}",
                    super::text::sanitize(&error)
                )),
            },
        }
    }

    fn apply_plugin_update(&mut self, update: crate::plugin_manager::PreparedUpdate) {
        let name = update.plugin.name.clone();
        let commit = update.plugin.commit.clone();
        match self.plugin_manager.apply_update(&update) {
            Ok(()) => {
                self.reload_plugins_after_change();
                self.show_message(format!(
                    "Updated plugin {} to {}",
                    super::text::sanitize(&name),
                    crate::plugin_manager::short_commit(&commit)
                ));
            }
            Err(error) => self.show_message(format!(
                "Cannot update plugin: {}",
                super::text::sanitize(&error)
            )),
        }
    }

    fn reload_plugins_after_change(&mut self) {
        self.plugins.set_lock(self.plugin_manager.lock().clone());
        match self
            .plugins
            .reload(&mut self.commands, &mut self.keymaps, &self.leaders, None)
        {
            Ok(warnings) => {
                self.rebuild_plugin_styles();
                self.refresh_command_completion();
                if !warnings.is_empty() {
                    self.show_pager(warnings);
                }
            }
            Err(error) => self.show_message(format!(
                "Cannot reload plugins: {}",
                super::text::sanitize(&error.to_string())
            )),
        }
    }
}
