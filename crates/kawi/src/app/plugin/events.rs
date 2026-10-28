use kawi_plugin::{PluginHostEvent, PluginStatus};

use super::super::App;

impl App {
    pub(crate) fn poll_plugins(&mut self) -> bool {
        let events = self.plugins.drain_events();
        let mut redraw = false;
        for event in events {
            redraw |= self.handle_plugin_event(event);
        }
        self.plugins.flush_activation_events();
        self.plugins
            .sync_plugin_state(&mut self.editor, &self.problems);
        redraw
    }

    fn handle_plugin_event(&mut self, event: PluginHostEvent) -> bool {
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
                self.show_message(format!("{plugin}: {error}"));
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
                }
                true
            }
            PluginHostEvent::Log { .. } => false,
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
            self.plugins.record_denial(plugin, capability);
            if !id.is_null() {
                let error = self.plugins.capability_error(plugin, capability);
                let _ = self.plugins.respond(plugin, id, Err(error));
            }
            return false;
        }
        if method == "ui.message" && !self.plugins.allow_message(plugin) {
            if !id.is_null() {
                let _ = self
                    .plugins
                    .respond(plugin, id, Ok(serde_json::Value::Null));
            }
            return false;
        }
        let redraw = matches!(method, "editor.apply_edit" | "ui.message");
        if method == "editor.apply_edit" {
            self.editor
                .set_change_origin(kawi_buffer::BufferChangeOrigin::Plugin(plugin.to_string()));
        }
        let result = self.handle_plugin_request(method, params);
        self.editor
            .set_change_origin(kawi_buffer::BufferChangeOrigin::User);
        if !id.is_null() {
            let _ = self.plugins.respond(plugin, id, result);
        }
        redraw
    }
}
