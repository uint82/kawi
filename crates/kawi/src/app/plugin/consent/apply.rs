use std::time::Instant;

use kawi_plugin::Capability;

use super::super::super::App;
use super::super::grant_file::GrantDecision;
use super::super::text::sanitize;
use super::queue::{GrantChoice, GrantKind, GrantRequest};

impl App {
    pub(super) fn apply_grant(&mut self, request: GrantRequest, choice: GrantChoice) {
        match choice {
            GrantChoice::Once => self.grant_once(&request),
            GrantChoice::Always => self.grant_always(&request),
            GrantChoice::Deny => self.deny_request(&request),
        }
        self.advance_grant();
    }

    pub(super) fn apply_batch(&mut self, request: GrantRequest) {
        let mut decisions = Vec::new();
        let mut allowed = Vec::new();
        let mut once = Vec::new();
        let mut denied = Vec::new();
        for item in &request.items {
            match item.choice {
                GrantChoice::Once => {
                    allowed.push(item.capability);
                    once.push(item.capability);
                }
                GrantChoice::Always => {
                    allowed.push(item.capability);
                    decisions.push(GrantDecision {
                        capability: item.capability,
                        allow: true,
                    });
                }
                GrantChoice::Deny => {
                    denied.push(item.capability);
                    decisions.push(GrantDecision {
                        capability: item.capability,
                        allow: false,
                    });
                }
            }
        }
        let persistent = decisions.iter().any(|decision| decision.allow);
        let saved = if persistent {
            match self.plugins.write_grants(&request.plugin, &decisions) {
                Ok(()) => {
                    let (_, restarted) = self.plugins.reload_permissions_without_restart();
                    self.deferred_plugin_restarts.extend(restarted);
                    true
                }
                Err(error) => {
                    let suffix = if request.kind == GrantKind::Call {
                        "cannot save grant; allowing once"
                    } else {
                        "cannot save grant"
                    };
                    self.show_grant_failure(&request, &error, suffix);
                    false
                }
            }
        } else {
            true
        };
        if saved {
            for capability in once {
                self.grant_once_capability(&request.plugin, capability);
            }
            for capability in &allowed {
                self.plugins.clear_denial(&request.plugin, *capability);
            }
            for capability in &denied {
                self.deny_capability(&request.plugin, *capability);
            }
            self.show_batch_message(&request, allowed.len(), denied.len());
        } else if request.kind == GrantKind::Call {
            for capability in allowed {
                self.allow_once_capability(&request.plugin, capability);
            }
            for capability in &denied {
                self.deny_capability(&request.plugin, *capability);
            }
        }
        self.advance_grant();
    }

    fn grant_once(&mut self, request: &GrantRequest) {
        self.grant_once_capability(&request.plugin, request.capability());
    }

    fn grant_once_capability(&mut self, plugin: &str, capability: Capability) {
        self.allow_once_capability(plugin, capability);
        self.show_message(format!(
            "Allowing next {} call from {}",
            capability.name(),
            sanitize(plugin)
        ));
    }

    fn allow_once_capability(&mut self, plugin: &str, capability: Capability) {
        self.plugins.allow_next(plugin, capability);
        self.plugins.clear_denial(plugin, capability);
    }

    fn grant_always(&mut self, request: &GrantRequest) {
        let decision = GrantDecision {
            capability: request.capability(),
            allow: true,
        };
        match self.plugins.write_grants(&request.plugin, &[decision]) {
            Ok(()) => {
                let (_, restarted) = self.plugins.reload_permissions_without_restart();
                self.deferred_plugin_restarts.extend(restarted);
                if self
                    .plugins
                    .has_capability(&request.plugin, request.capability())
                {
                    self.plugins
                        .clear_denial(&request.plugin, request.capability());
                    self.show_message(format!(
                        "Granted {} to {}",
                        request.capability().name(),
                        sanitize(&request.plugin)
                    ));
                    return;
                }
                self.handle_grant_failure(request, "grant was not applied");
            }
            Err(error) => self.handle_grant_failure(request, &error),
        }
    }

    fn handle_grant_failure(&mut self, request: &GrantRequest, reason: &str) {
        if request.kind == GrantKind::Startup {
            self.show_grant_failure(request, reason, "cannot save grant");
            return;
        }
        self.plugins
            .allow_next(&request.plugin, request.capability());
        self.plugins
            .clear_denial(&request.plugin, request.capability());
        self.show_grant_failure(request, reason, "cannot save grant; allowing once");
    }

    fn show_grant_failure(&mut self, request: &GrantRequest, reason: &str, suffix: &str) {
        self.plugins.push_log(
            &request.plugin,
            &format!("cannot save grant: {}", sanitize(reason)),
        );
        self.show_message(format!("{}: {suffix}", sanitize(&request.plugin)));
    }

    pub(super) fn deny_request(&mut self, request: &GrantRequest) {
        for item in &request.items {
            self.deny_capability(&request.plugin, item.capability);
        }
        self.show_batch_message(request, 0, request.items.len());
    }

    fn deny_capability(&mut self, plugin: &str, capability: Capability) {
        self.plugins
            .push_log(plugin, &format!("denied by user: {}", capability.name()));
    }

    fn show_batch_message(&mut self, request: &GrantRequest, allowed: usize, denied: usize) {
        let plugin = sanitize(&request.plugin);
        match (allowed, denied) {
            (0, 0) => {}
            (0, _) => self.show_message(format!("Denied {denied} for {plugin}")),
            (_, 0) => self.show_message(format!("Granted {allowed} to {plugin}")),
            _ => self.show_message(format!(
                "Granted {allowed} and denied {denied} for {plugin}"
            )),
        }
    }

    pub(super) fn advance_grant(&mut self) {
        if self.plugin_grants.activate_next(Instant::now()) {
            self.show_grant_prompt_message();
        }
        self.flush_deferred_plugin_restarts();
    }

    pub(super) fn flush_deferred_plugin_restarts(&mut self) {
        let ready = self
            .deferred_plugin_restarts
            .iter()
            .filter(|plugin| !self.plugin_grants.has_plugin_requests(plugin))
            .cloned()
            .collect::<Vec<_>>();
        for plugin in ready {
            self.deferred_plugin_restarts.remove(&plugin);
            let _ = self.plugins.restart(Some(&plugin));
        }
    }
}
