use kawi_plugin::PluginHostEvent;

use super::super::App;

impl App {
    pub(crate) fn poll_plugins(&mut self) -> bool {
        let events = self.plugins.drain_events();
        let mut redraw = false;
        for event in events {
            redraw |= self.handle_plugin_event(event);
        }
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
            PluginHostEvent::StatusChanged { .. } => true,
            PluginHostEvent::Log { .. } => false,
        }
    }

    fn handle_plugin_host_request(
        &mut self,
        plugin: &str,
        id: serde_json::Value,
        method: &str,
        params: serde_json::Value,
    ) -> bool {
        let redraw = matches!(method, "editor.apply_edit" | "ui.message");
        if method == "editor.apply_edit" {
            self.editor
                .set_change_origin(kawi_buffer::BufferChangeOrigin::Plugin(plugin.to_string()));
        }
        let result = self.handle_plugin_request(method, params);
        self.editor
            .set_change_origin(kawi_buffer::BufferChangeOrigin::User);
        let _ = self.plugins.respond(plugin, id, result);
        redraw
    }
}
