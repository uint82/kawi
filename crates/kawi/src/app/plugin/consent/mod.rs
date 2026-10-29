mod queue;

pub(super) use self::queue::GrantKind;
pub(crate) use self::queue::GrantQueue;
use std::time::Instant;

use self::queue::{
    changed_request_summary, navigation_key, Enqueue, GrantChoice, GrantPrompt, GrantRequest,
};
use crossterm::event::{KeyCode, KeyEvent};
use kawi_plugin::Capability;

use super::super::App;
use super::text::sanitize;

impl App {
    pub(super) fn request_plugin_grant(
        &mut self,
        plugin: &str,
        capability: Capability,
        kind: GrantKind,
    ) {
        if self.plugins.has_capability(plugin, capability)
            || self.plugins.request_scope(plugin, capability).is_none()
        {
            return;
        }
        let kind = if self.plugins.is_wasm(plugin) {
            GrantKind::Startup
        } else {
            kind
        };
        let request = GrantRequest {
            plugin: plugin.to_string(),
            capability,
            changed: self.plugins.is_changed(plugin),
            changed_requests: self.plugins.changed_requests(plugin),
            kind,
        };
        match self.plugin_grants.enqueue(request, Instant::now()) {
            Enqueue::Activated => self.show_grant_prompt_message(),
            Enqueue::Full => self
                .plugins
                .push_log(plugin, "grant prompt queue full; request denied"),
            Enqueue::Queued | Enqueue::Duplicate => {}
        }
    }

    pub(crate) fn handle_plugin_grant_key(&mut self, key: KeyEvent) -> bool {
        if !GrantPrompt::accepts_key(key) {
            return false;
        }
        let Some(mut prompt) = self.plugin_grants.take_active() else {
            return false;
        };
        if let Some(delta) = navigation_key(key) {
            prompt.move_by(delta);
            self.plugin_grants.restore_active(prompt);
            return true;
        }
        match key.code {
            KeyCode::Esc => {
                self.deny_grant(prompt.request());
                self.advance_grant();
            }
            KeyCode::Enter => {
                let request = prompt.request().clone();
                let choice = prompt.choice();
                self.apply_grant(request, choice);
            }
            _ => self.plugin_grants.restore_active(prompt),
        }
        true
    }

    pub(super) fn expire_plugin_grants(&mut self) {
        let now = Instant::now();
        if let Some(request) = self.plugin_grants.expire(now) {
            self.plugins
                .push_log(&request.plugin, "grant prompt expired; denied");
            self.show_message(format!(
                "Grant prompt expired: {} {}",
                sanitize(&request.plugin),
                request.capability.name()
            ));
        }
        if self.plugin_grants.activate_next(now) {
            self.show_grant_prompt_message();
        }
    }

    pub(crate) fn grant_prompt_items(&self) -> Option<(&[String], usize)> {
        self.plugin_grants
            .active()
            .map(|prompt| (self.plugin_grants.items(), prompt.selected()))
    }

    fn apply_grant(&mut self, request: GrantRequest, choice: GrantChoice) {
        match choice {
            GrantChoice::Once => self.grant_once(request),
            GrantChoice::Always => self.grant_always(request),
            GrantChoice::Deny => self.deny_grant(&request),
        }
        self.advance_grant();
    }

    fn grant_once(&mut self, request: GrantRequest) {
        self.plugins.allow_next(&request.plugin, request.capability);
        self.plugins
            .clear_denial(&request.plugin, request.capability);
        self.show_message(format!(
            "Allowing next {} call from {}",
            request.capability.name(),
            sanitize(&request.plugin)
        ));
    }

    fn grant_always(&mut self, request: GrantRequest) {
        match self
            .plugins
            .write_grant(&request.plugin, request.capability)
        {
            Ok(()) => {
                self.plugins.reload_permissions();
                if self
                    .plugins
                    .has_capability(&request.plugin, request.capability)
                {
                    self.plugins
                        .clear_denial(&request.plugin, request.capability);
                    self.show_message(format!(
                        "Granted {} to {}",
                        request.capability.name(),
                        sanitize(&request.plugin)
                    ));
                    return;
                }
                self.handle_grant_failure(&request, "grant was not applied");
            }
            Err(error) => self.handle_grant_failure(&request, &error),
        }
    }

    fn handle_grant_failure(&mut self, request: &GrantRequest, reason: &str) {
        if request.kind == GrantKind::Startup {
            self.show_grant_failure(request, reason, "cannot save grant");
            return;
        }
        self.plugins.allow_next(&request.plugin, request.capability);
        self.plugins
            .clear_denial(&request.plugin, request.capability);
        self.show_grant_failure(request, reason, "cannot save grant; allowing once");
    }

    fn show_grant_failure(&mut self, request: &GrantRequest, reason: &str, suffix: &str) {
        self.plugins.push_log(
            &request.plugin,
            &format!("cannot save grant: {}", sanitize(reason)),
        );
        self.show_message(format!("{}: {suffix}", sanitize(&request.plugin)));
    }

    fn deny_grant(&mut self, request: &GrantRequest) {
        self.plugins.push_log(
            &request.plugin,
            &format!("denied by user: {}", request.capability.name()),
        );
        self.show_message(format!(
            "Denied {} for {}",
            request.capability.name(),
            sanitize(&request.plugin)
        ));
    }

    fn advance_grant(&mut self) {
        if self.plugin_grants.activate_next(Instant::now()) {
            self.show_grant_prompt_message();
        }
    }

    fn show_grant_prompt_message(&mut self) {
        let Some(prompt) = self.plugin_grants.active() else {
            return;
        };
        let request = prompt.request();
        self.show_message(format!(
            "{} requests {}.{}",
            sanitize(&request.plugin),
            request.capability.name(),
            changed_request_summary(request)
        ));
    }
}
