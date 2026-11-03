use kawi_plugin::{PluginHostEvent, PluginStatus, RpcError};

use super::super::App;

impl App {
    pub(crate) fn poll_plugins(&mut self) -> bool {
        let dropped = self.plugins.expire_queued();
        self.log_queue_drops(&dropped, "queue wait exceeded");
        let events = self.plugins.drain_events();
        let mut redraw = false;
        for event in events {
            redraw |= self.handle_plugin_event(event);
        }
        self.plugins.flush_activation_events();
        self.plugins
            .sync_plugin_state(&mut self.editor, &mut self.problems);
        self.plugin_decorations.retain_live(&self.editor);
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
            PluginHostEvent::InvocationFinished { plugin, result, .. } => {
                let dropped = self.plugins.finish_invocation(&plugin, &result);
                self.log_queue_drops(&dropped, "plugin restarted");
                let redraw = if let Err(error) = &result {
                    self.handle_plugin_invocation_error(&plugin, "", error);
                    true
                } else {
                    false
                };
                self.advance_plugin_queue(&plugin);
                redraw
            }
            PluginHostEvent::StatusChanged { plugin, status } => {
                if matches!(
                    status,
                    PluginStatus::Stopped
                        | PluginStatus::Backoff { .. }
                        | PluginStatus::Disabled
                        | PluginStatus::Error(_)
                ) {
                    let dropped = self.plugins.drop_queues(Some(&plugin));
                    self.log_queue_drops(&dropped, "plugin stopped");
                    self.plugins.stop_jobs(&plugin);
                    self.plugins.clear_plugin_state(&plugin);
                    self.plugin_decorations.remove_plugin(&plugin);
                    self.plugin_statusline.remove_plugin(&plugin);
                    self.problems.clear_plugin(&plugin);
                    if let Some((stopped, request)) = self.plugin_ui.remove_plugin(&plugin) {
                        let _ = self.plugins.respond(
                            &stopped,
                            request,
                            Err(RpcError::unavailable("plugin stopped")),
                        );
                    }
                }
                true
            }
            PluginHostEvent::CapabilityRequired { plugin, capability } => {
                if !self.plugins.has_capability(&plugin, capability) {
                    self.plugins.record_denial(&plugin, capability);
                }
                false
            }
            PluginHostEvent::JobEvent {
                plugin,
                event,
                payload,
            } => {
                let _ = self.plugins.notify_job(&plugin, &event, payload);
                false
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
            self.plugins.record_denial(plugin, capability);
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
            "editor.apply_edit"
                | "editor.execute"
                | "editor.set_diagnostics"
                | "editor.set_decorations"
                | "ui.set_statusline"
                | "ui.message"
                | "ui.input"
                | "ui.select"
                | "ui.confirm"
                | "ui.progress_start"
                | "ui.progress_update"
                | "ui.progress_finish"
        );
        if matches!(method, "editor.apply_edit" | "editor.execute") {
            self.editor
                .set_change_origin(kawi_buffer::BufferChangeOrigin::Plugin(plugin.to_string()));
        }
        if matches!(method, "ui.input" | "ui.select" | "ui.confirm") {
            if let Some(result) = self.handle_plugin_prompt_request(plugin, &id, method, params) {
                if !id.is_null() {
                    let _ = self.plugins.respond(plugin, id, result);
                }
            }
            return true;
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
