mod apply;
mod prompt;
mod queue;
mod request;

pub(super) use self::queue::GrantKind;
pub(crate) use self::queue::GrantQueue;
use self::queue::{GrantItem, GrantRequest};
use std::time::Instant;

use self::queue::{changed_request_summary, navigation_key, Enqueue, GrantPrompt};
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
        let Some(request_hash) = self.plugins.request_hash(plugin) else {
            return;
        };
        let kind = if self.plugins.is_wasm(plugin) {
            GrantKind::Startup
        } else {
            kind
        };
        let mut capabilities = self.plugins.changed_capabilities(plugin);
        if !capabilities.contains(&capability) {
            capabilities.push(capability);
        }
        let items = capabilities
            .into_iter()
            .map(|capability| {
                GrantItem::new(
                    capability,
                    self.plugins.change_kind(plugin, capability),
                    kind,
                )
            })
            .collect();
        let request = GrantRequest {
            plugin: plugin.to_string(),
            changed: self.plugins.is_changed(plugin),
            changed_requests: self.plugins.changed_requests(plugin),
            kind,
            request_hash,
            items,
        };
        match self.plugin_grants.enqueue(request, Instant::now()) {
            Enqueue::Activated | Enqueue::Merged => self.show_grant_prompt_message(),
            Enqueue::Full => self
                .plugins
                .push_log(plugin, "grant prompt queue full; request denied"),
            Enqueue::Queued | Enqueue::Duplicate | Enqueue::Suppressed => {}
        }
    }

    pub(crate) fn handle_plugin_grant_key(&mut self, key: KeyEvent) {
        if !GrantPrompt::accepts_key(key) {
            return;
        }
        let Some(mut prompt) = self.plugin_grants.take_active() else {
            return;
        };
        if let Some(delta) = navigation_key(key) {
            prompt.move_by(delta);
            self.plugin_grants.restore_active(prompt);
            return;
        }
        if prompt.is_batch() && update_batch_choice(&mut prompt, key) {
            self.plugin_grants.restore_active(prompt);
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.deny_request(prompt.request());
                self.advance_grant();
            }
            KeyCode::Enter => self.apply_grant_prompt(prompt),
            _ => self.plugin_grants.restore_active(prompt),
        }
    }

    fn apply_grant_prompt(&mut self, prompt: GrantPrompt) {
        if prompt.is_batch() {
            self.apply_batch(prompt.into_request());
        } else {
            let choice = prompt.choice();
            self.apply_grant(prompt.into_request(), choice);
        }
    }

    pub(super) fn expire_plugin_grants(&mut self) {
        let now = Instant::now();
        if let Some(request) = self.plugin_grants.expire(now) {
            self.deny_request(&request);
            self.show_message(format!(
                "Grant prompt expired: {}",
                sanitize(&request.plugin)
            ));
        }
        if self.plugin_grants.activate_next(now) {
            self.show_grant_prompt_message();
        }
        self.flush_deferred_plugin_restarts();
    }

    pub(crate) fn grant_prompt_items(&self) -> Option<(&[String], usize)> {
        self.plugin_grants
            .active()
            .map(|prompt| (self.plugin_grants.items(), prompt.selected()))
    }

    fn show_grant_prompt_message(&mut self) {
        let Some(prompt) = self.plugin_grants.active() else {
            return;
        };
        let request = prompt.request();
        if prompt.is_batch() {
            self.show_message(format!(
                "{} requests {} changes.",
                sanitize(&request.plugin),
                request.items.len()
            ));
            return;
        }
        self.show_message(format!(
            "{} requests {}.{}",
            sanitize(&request.plugin),
            request.capability().name(),
            changed_request_summary(request)
        ));
    }
}

fn update_batch_choice(prompt: &mut GrantPrompt, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('a') => prompt.allow_all(),
        KeyCode::Char('d') => prompt.deny_all(),
        KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right => prompt.toggle_selected(),
        _ => return false,
    }
    true
}
