use kawi_plugin::PluginStatus;

use super::super::command_text::split_command;
use super::super::App;

pub(crate) const SUBCOMMANDS: &[&str] = &[
    "install", "update", "remove", "disable", "enable", "list", "reload", "restart", "logs",
    "inspect", "settings", "cancel", "panel",
];

impl App {
    pub(crate) fn handle_plugin_command(&mut self, text: &str) {
        let text = text.trim();
        let (name, rest) = split_command(text);
        match name {
            "install" => self.install_plugin(rest),
            "update" => self.update_plugin(rest),
            "remove" => self.remove_plugin(rest),
            "disable" => self.set_plugin_enabled(rest, false),
            "enable" => self.set_plugin_enabled(rest, true),
            "list" => self.list_plugins(),
            "reload" => self.restart_plugins(""),
            "restart" => self.restart_plugins(rest),
            "logs" => self.show_plugin_logs(rest),
            "inspect" => self.inspect_plugin(rest),
            "settings" => self.open_plugin_settings(rest),
            "cancel" => {
                let _ = self.cancel_plugin_progress();
            }
            "panel" => self.handle_plugin_panel_command(rest),
            _ => self.show_message(
                "Usage: plugin install <owner/repo[@ref]> | update <plugin> | remove <plugin> | disable <plugin> | enable <plugin> | list | reload | restart [name] | logs [name] | inspect <plugin> | settings [plugin] | cancel | panel [close]"
                    .to_string(),
            ),
        }
    }

    fn list_plugins(&mut self) {
        let mut lines = vec![format!(
            "{:<16} {:<8} {:<8} {:<40} {:<20} {:<24} {}",
            "name", "version", "runtime", "status", "source", "host apis", "commands"
        )];
        lines.extend(self.plugins.plugins().iter().map(|plugin| {
            let name = plugin.manifest.name.as_str();
            let status = self
                .plugins
                .status(name)
                .map(status_label)
                .unwrap_or_else(|| "stopped".to_string());
            let depth = self.plugins.queue_depth(name);
            let status = if depth > 0 {
                format!("busy ({depth} queued)")
            } else {
                status
            };
            let status = super::text::sanitize(&status);
            let commands = plugin
                .manifest
                .commands
                .iter()
                .map(|command| plugin.command_name(command))
                .collect::<Vec<_>>()
                .join(", ");
            let capabilities = self.plugins.capability_lines(name).join("; ");
            let source = self
                .plugin_manager
                .lock()
                .plugins
                .get(name)
                .map(|locked| {
                    format!(
                        "{}@{}",
                        locked.source,
                        crate::plugin_manager::short_commit(&locked.commit)
                    )
                })
                .unwrap_or_else(|| "development".to_string());
            format!(
                "{:<16} {:<8} {:<8} {:<40} {:<20} {:<24} {}",
                name,
                super::text::sanitize(&plugin.manifest.version),
                plugin.manifest.runtime.label(),
                status,
                source,
                capabilities,
                commands
            )
        }));
        for plugin in self.plugins.plugins() {
            let name = plugin.manifest.name.as_str();
            for line in self.plugins.denial_records(name) {
                lines.push(format!("{name}: {line}"));
            }
        }
        for (name, locked) in &self.plugin_manager.lock().plugins {
            if self.plugins.contains(name) {
                continue;
            }
            let source = format!(
                "{}@{}",
                locked.source,
                crate::plugin_manager::short_commit(&locked.commit)
            );
            lines.push(format!(
                "{:<16} {:<8} {:<8} {:<40} {:<20} {:<24} {}",
                name,
                "-",
                "wasm",
                if locked.enabled {
                    "not loaded"
                } else {
                    "disabled"
                },
                super::text::sanitize(&source),
                "-",
                "no commands"
            ));
        }
        if lines.is_empty() {
            self.show_message("No plugins installed".to_string());
        } else {
            self.show_pager(lines);
        }
    }

    fn restart_plugins(&mut self, name: &str) {
        let name = name.trim();
        let target = (!name.is_empty()).then_some(name);
        match self
            .plugins
            .reload(&mut self.commands, &mut self.keymaps, &self.leaders, target)
        {
            Ok(warnings) => {
                self.rebuild_plugin_styles();
                self.refresh_command_completion();
                if !warnings.is_empty() {
                    self.show_pager(warnings);
                    return;
                }
                let message = match target {
                    Some(name) if self.plugins.contains(name) => {
                        format!("Plugin reloaded: {name}")
                    }
                    Some(name) => format!("Plugin removed: {name}"),
                    None => "Plugins reloaded".to_string(),
                };
                self.show_message(message);
            }
            Err(error) => self.show_message(format!(
                "Cannot reload plugin: {}",
                super::text::sanitize(&error.to_string())
            )),
        }
    }

    fn inspect_plugin(&mut self, name: &str) {
        let name = name.trim();
        let Some(plugin) = self
            .plugins
            .plugins()
            .iter()
            .find(|plugin| plugin.manifest.name == name)
        else {
            self.show_message(format!("Unknown plugin: {}", super::text::sanitize(name)));
            return;
        };
        let mut lines = vec![
            format!("name: {}", plugin.manifest.name),
            format!(
                "version: {}",
                super::text::sanitize(&plugin.manifest.version)
            ),
            format!("runtime: {}", plugin.manifest.runtime.label()),
        ];
        if let Some(component) = plugin.manifest.component.as_deref() {
            lines.push(format!("component: {}", super::text::sanitize(component)));
        }
        if let Some(hash) = plugin.component_hash() {
            lines.push(format!("sha256: {}", super::text::sanitize(hash)));
        }
        if let Some(hash) = plugin.declared_hash() {
            lines.push(format!("declared sha256: {}", super::text::sanitize(hash)));
        }
        if let Some(reason) = plugin.disabled_reason.as_deref() {
            lines.push(format!("disabled: {}", super::text::sanitize(reason)));
        }
        for (capability, scope) in plugin.manifest.capability_requests.iter() {
            lines.push(format!(
                "declares: {} {}",
                capability.name(),
                serde_json::Value::Object(scope.to_json_object())
            ));
        }
        if !plugin.manifest.settings.is_empty() {
            let settings = plugin.manifest.settings.keys().cloned().collect::<Vec<_>>();
            lines.push(format!("settings: {}", settings.join(", ")));
        }
        #[cfg(feature = "wasm")]
        if plugin.manifest.runtime == kawi_plugin::PluginRuntime::Wasm
            && plugin.disabled_reason.is_none()
        {
            lines.extend(inspect_wasm_component(plugin));
        }
        self.show_pager(lines);
    }

    fn show_plugin_logs(&mut self, name: &str) {
        let name = name.trim();
        let lines = if name.is_empty() {
            self.all_plugin_logs()
        } else {
            self.plugins.logs(name)
        }
        .into_iter()
        .map(|line| super::text::sanitize(&line))
        .collect::<Vec<_>>();
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

#[cfg(feature = "wasm")]
fn inspect_wasm_component(plugin: &kawi_plugin::LoadedPlugin) -> Vec<String> {
    let Some(path) = plugin.component_path() else {
        return vec!["component: missing path".to_string()];
    };
    let requested = plugin
        .manifest
        .capability_requests
        .iter()
        .map(|(capability, _)| capability)
        .collect::<Vec<_>>();
    match kawi_plugin::WasmRuntime::new()
        .and_then(|runtime| runtime.inspect(&path, plugin.component_hash(), &requested))
    {
        Ok(inspection) => {
            let mut lines = vec![format!("component hash: {}", inspection.hash)];
            lines.push("imports:".to_string());
            lines.extend(
                inspection
                    .imports
                    .into_iter()
                    .map(|value| format!("  {value}")),
            );
            lines
        }
        Err(error) => vec![format!("imports: unavailable: {error}")],
    }
}
