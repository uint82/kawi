use anyhow::Result;
use kawi_plugin::PluginError;
use serde_json::Value;

use super::super::App;
use super::consent::GrantKind;
use super::queue::{capability_from_error, QueueDrop, QueueOutcome};
use super::text::sanitize;

impl App {
    pub(crate) fn run_plugin_command(
        &mut self,
        target: u32,
        args: Option<toml::Value>,
    ) -> Result<bool> {
        let Some((plugin, command)) = self
            .plugins
            .target(target)
            .map(|(plugin, command)| (plugin.to_string(), command.to_string()))
        else {
            self.show_message("Unknown plugin command".to_string());
            return Ok(false);
        };
        let args = match args {
            Some(args) => match serde_json::to_value(args) {
                Ok(args) => args,
                Err(error) => {
                    self.show_message(format!("{plugin}.{command}: {error}"));
                    return Ok(false);
                }
            },
            None => Value::Null,
        };
        match self.plugins.enqueue_invocation(target, args.clone()) {
            Ok(QueueOutcome::Start) => self.dispatch_plugin_command(&plugin, &command, args),
            Ok(QueueOutcome::Queued { depth } | QueueOutcome::Coalesced { depth }) => {
                if self.plugins.allow_busy_message(&plugin) {
                    self.show_message(format!("{}: busy, {} queued", sanitize(&plugin), depth));
                }
            }
            Err(error) => self.handle_plugin_invocation_error(&plugin, &command, &error),
        }
        Ok(false)
    }

    pub(super) fn dispatch_plugin_command(&mut self, plugin: &str, command: &str, args: Value) {
        let context = self.plugin_context();
        if let Err(error) = self.plugins.invoke_named(plugin, command, args, context) {
            let failed = Err(error.clone());
            let dropped = self.plugins.finish_invocation(plugin, &failed);
            self.log_queue_drops(&dropped, "plugin unavailable");
            self.handle_plugin_invocation_error(plugin, command, &error);
            self.advance_plugin_queue(plugin);
        }
    }

    pub(super) fn advance_plugin_queue(&mut self, plugin: &str) {
        let dropped = self.plugins.expire_queued();
        self.log_queue_drops(&dropped, "queue wait exceeded");
        loop {
            let Some(queued) = self.plugins.take_queued(plugin) else {
                return;
            };
            if let Some(capability) = self.plugins.command_denied(&queued.plugin, &queued.command) {
                let error = self.plugins.command_error(&queued.plugin, capability);
                self.handle_plugin_invocation_error(&queued.plugin, &queued.command, &error);
                self.plugins.abandon_invocation(&queued.plugin);
                continue;
            }
            self.dispatch_plugin_command(&queued.plugin, &queued.command, queued.args);
            return;
        }
    }

    pub(super) fn handle_plugin_invocation_error(
        &mut self,
        plugin: &str,
        command: &str,
        error: &PluginError,
    ) {
        if let Some(capability) = capability_from_error(error) {
            let first = self.plugins.record_denial(plugin, capability);
            self.request_plugin_grant(plugin, capability, GrantKind::Call);
            if first {
                self.show_message(format!(
                    "{}: capability denied: {}",
                    sanitize(plugin),
                    capability.name()
                ));
            }
            return;
        }
        if self.plugin_grants.is_active() {
            self.plugins
                .push_log(plugin, "request failed while a grant prompt is open");
            return;
        }
        let label = if command.is_empty() {
            plugin.to_string()
        } else {
            format!("{plugin}.{command}")
        };
        self.show_message(format!(
            "{}: {}",
            sanitize(&label),
            sanitize(&error.to_string())
        ));
    }

    pub(super) fn log_queue_drops(&mut self, drops: &[QueueDrop], reason: &str) {
        for drop in drops {
            self.plugins.push_log(
                &drop.plugin,
                &format!(
                    "queued command dropped: {} ({reason})",
                    sanitize(&drop.command)
                ),
            );
        }
    }
}
