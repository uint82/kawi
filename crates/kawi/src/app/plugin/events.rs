use kawi_plugin::{PluginHostEvent, PluginStatus};

use super::super::App;
use super::text::sanitize;

impl App {
    pub(crate) fn poll_plugins(&mut self) -> bool {
        let events = self.plugins.drain_events();
        let mut redraw = false;
        for event in events {
            redraw |= self.handle_plugin_event(event);
        }
        if let Some(warnings) = self.plugins.poll_permission_changes() {
            redraw = true;
            if warnings.is_empty() {
                self.show_message("Plugin permissions reloaded".to_string());
            } else {
                self.show_pager(warnings);
            }
        }
        self.plugins.flush_activation_events();
        self.plugins
            .sync_plugin_state(&mut self.editor, &mut self.problems);
        self.expire_plugin_grants();
        redraw
    }

    pub(super) fn handle_plugin_event(&mut self, event: PluginHostEvent) -> bool {
        match event {
            PluginHostEvent::HostRequest {
                plugin,
                id,
                method,
                params,
            } => self.handle_plugin_host_request(&plugin, id, &method, params),
            PluginHostEvent::InvocationFinished {
                plugin,
                result: Err(error),
                ..
            } => {
                if self.plugin_grants.is_active() {
                    self.plugins
                        .push_log(&plugin, "request failed while a grant prompt is open");
                } else {
                    self.show_message(format!(
                        "{}: {}",
                        sanitize(&plugin),
                        sanitize(&error.to_string())
                    ));
                }
                true
            }
            PluginHostEvent::InvocationFinished { result: Ok(_), .. } => false,
            PluginHostEvent::StatusChanged { plugin, status } => {
                if matches!(
                    status,
                    PluginStatus::Stopped
                        | PluginStatus::Backoff { .. }
                        | PluginStatus::Disabled
                        | PluginStatus::Error(_)
                ) {
                    self.plugins.clear_plugin_state(&plugin);
                    self.problems.clear_plugin(&plugin);
                }
                true
            }
            PluginHostEvent::Log { .. } => false,
            PluginHostEvent::SandboxApplied { plugin } => {
                self.plugins.mark_sandboxed(&plugin);
                false
            }
        }
    }

    pub(super) fn handle_plugin_host_request(
        &mut self,
        plugin: &str,
        id: serde_json::Value,
        method: &str,
        params: serde_json::Value,
    ) -> bool {
        if let Some(capability) = self.plugins.denied_capability(plugin, method) {
            if self.plugins.take_allow_next(plugin, capability) {
                self.plugins.clear_denial(plugin, capability);
                return self.handle_authorized_request(plugin, id, method, params);
            }
            self.plugins.record_denial(plugin, capability);
            self.request_plugin_grant(plugin, capability);
            if !id.is_null() {
                let error = self.plugins.capability_error(plugin, capability);
                let _ = self.plugins.respond(plugin, id, Err(error));
            }
            return false;
        }
        self.handle_authorized_request(plugin, id, method, params)
    }

    fn handle_authorized_request(
        &mut self,
        plugin: &str,
        id: serde_json::Value,
        method: &str,
        params: serde_json::Value,
    ) -> bool {
        if method == "ui.message" && !self.plugins.allow_message(plugin) {
            if !id.is_null() {
                let _ = self
                    .plugins
                    .respond(plugin, id, Ok(serde_json::Value::Null));
            }
            return false;
        }
        let redraw = matches!(
            method,
            "editor.apply_edit" | "editor.set_diagnostics" | "ui.message"
        );
        if method == "editor.apply_edit" {
            self.editor
                .set_change_origin(kawi_buffer::BufferChangeOrigin::Plugin(plugin.to_string()));
        }
        let result = self.handle_plugin_request(plugin, method, params);
        self.editor
            .set_change_origin(kawi_buffer::BufferChangeOrigin::User);
        if !id.is_null() {
            let _ = self.plugins.respond(plugin, id, result);
        }
        redraw
    }
}
