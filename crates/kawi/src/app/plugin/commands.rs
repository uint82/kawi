use kawi_plugin::PluginStatus;

use super::super::command_text::split_command;
use super::super::App;

impl App {
    pub(crate) fn handle_plugin_command(&mut self, text: &str) {
        let text = text.trim();
        let (name, rest) = split_command(text);
        match name {
            "list" => self.list_plugins(),
            "restart" => self.restart_plugins(rest),
            "logs" => self.show_plugin_logs(rest),
            _ => self.show_message("Usage: plugin list | restart [name] | logs [name]".to_string()),
        }
    }

    fn list_plugins(&mut self) {
        let lines = self
            .plugins
            .plugins()
            .iter()
            .map(|plugin| {
                let name = plugin.manifest.name.as_str();
                let status = self
                    .plugins
                    .status(name)
                    .map(status_label)
                    .unwrap_or_else(|| "stopped".to_string());
                let commands = plugin
                    .manifest
                    .commands
                    .iter()
                    .map(|command| plugin.command_name(command))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "{:<16} {:<8} {:<20} {}",
                    name, plugin.manifest.version, status, commands
                )
            })
            .collect::<Vec<_>>();
        if lines.is_empty() {
            self.show_message("No plugins installed".to_string());
        } else {
            self.show_pager(lines);
        }
    }

    fn restart_plugins(&mut self, name: &str) {
        let name = name.trim();
        let target = (!name.is_empty()).then_some(name);
        match self.plugins.restart(target) {
            Ok(()) => {
                let message = match target {
                    Some(name) => format!("Plugin restarted: {name}"),
                    None => "Plugins restarted".to_string(),
                };
                self.show_message(message);
            }
            Err(error) => self.show_message(format!("Cannot restart plugin: {error}")),
        }
    }

    fn show_plugin_logs(&mut self, name: &str) {
        let name = name.trim();
        let lines = if name.is_empty() {
            self.all_plugin_logs()
        } else {
            self.plugins.logs(name)
        };
        if lines.is_empty() {
            self.show_message("No plugin logs".to_string());
        } else {
            self.show_pager(lines);
        }
    }

    fn all_plugin_logs(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for plugin in self.plugins.plugins() {
            let name = plugin.manifest.name.as_str();
            for line in self.plugins.logs(name) {
                lines.push(format!("{name}: {line}"));
            }
        }
        lines
    }
}

fn status_label(status: PluginStatus) -> String {
    match status {
        PluginStatus::Stopped => "stopped".to_string(),
        PluginStatus::Starting => "starting".to_string(),
        PluginStatus::Ready => "ready".to_string(),
        PluginStatus::Backoff { remaining_ms } => format!("retry {remaining_ms}ms"),
        PluginStatus::Disabled => "disabled".to_string(),
        PluginStatus::Error(message) => format!("error: {message}"),
    }
}
