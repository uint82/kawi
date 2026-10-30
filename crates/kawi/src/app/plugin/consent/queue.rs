use std::collections::{BTreeSet, VecDeque};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
#[cfg(test)]
use kawi_plugin::Capability;

use super::super::text::sanitize;
pub(super) use super::prompt::{GrantChoice, GrantPrompt};
pub(crate) use super::request::GrantKind;
use super::request::StartupKey;
pub(super) use super::request::{GrantItem, GrantRequest};

const MAX_PENDING: usize = 8;
pub(super) const PROMPT_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Enqueue {
    Activated,
    Merged,
    Queued,
    Duplicate,
    Suppressed,
    Full,
}

#[derive(Debug, Default)]
pub(crate) struct GrantQueue {
    active: Option<GrantPrompt>,
    pending: VecDeque<GrantRequest>,
    seen_startup: BTreeSet<StartupKey>,
}

impl GrantQueue {
    pub(super) fn enqueue(&mut self, request: GrantRequest, now: Instant) -> Enqueue {
        let startup_key = request.startup_key();
        if startup_key
            .as_ref()
            .is_some_and(|key| self.seen_startup.contains(key))
        {
            return Enqueue::Suppressed;
        }
        if let Some(outcome) = self.merge_active(&request) {
            return outcome;
        }
        if let Some(outcome) = self.merge_pending(&request) {
            return outcome;
        }
        if self.active.is_none() {
            self.remember_startup(startup_key);
            self.active = Some(GrantPrompt::new(request, now));
            return Enqueue::Activated;
        }
        if self.pending.len() >= MAX_PENDING {
            return Enqueue::Full;
        }
        self.remember_startup(startup_key);
        self.pending.push_back(request);
        Enqueue::Queued
    }

    fn merge_active(&mut self, request: &GrantRequest) -> Option<Enqueue> {
        let prompt = self.active.as_mut()?;
        if !prompt.request().same_batch(request) {
            return None;
        }
        let added = prompt.merge(request.clone());
        let key = added.then(|| prompt.request().startup_key()).flatten();
        self.remember_startup(key);
        Some(if added {
            Enqueue::Merged
        } else {
            Enqueue::Duplicate
        })
    }

    fn merge_pending(&mut self, request: &GrantRequest) -> Option<Enqueue> {
        let pending = self
            .pending
            .iter_mut()
            .find(|pending| pending.same_batch(request))?;
        let added = pending.merge(request);
        let key = added.then(|| pending.startup_key()).flatten();
        self.remember_startup(key);
        Some(if added {
            Enqueue::Merged
        } else {
            Enqueue::Duplicate
        })
    }

    pub(super) fn has_plugin_requests(&self, plugin: &str) -> bool {
        self.active
            .as_ref()
            .is_some_and(|prompt| prompt.request().plugin == plugin)
            || self.pending.iter().any(|pending| pending.plugin == plugin)
    }

    #[cfg(test)]
    pub(crate) fn active_capability(&self) -> Option<Capability> {
        self.active
            .as_ref()
            .and_then(|prompt| prompt.request().items.first())
            .map(|item| item.capability)
    }

    #[cfg(all(test, feature = "wasm"))]
    pub(crate) fn active_capabilities(&self) -> Vec<Capability> {
        self.active
            .as_ref()
            .map(|prompt| {
                prompt
                    .request()
                    .items
                    .iter()
                    .map(|item| item.capability)
                    .collect()
            })
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(crate) fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub(super) fn active(&self) -> Option<&GrantPrompt> {
        self.active.as_ref()
    }

    pub(super) fn take_active(&mut self) -> Option<GrantPrompt> {
        self.active.take()
    }

    pub(super) fn restore_active(&mut self, prompt: GrantPrompt) {
        self.active = Some(prompt);
    }

    pub(super) fn items(&self) -> &[String] {
        self.active
            .as_ref()
            .map(GrantPrompt::labels)
            .unwrap_or_default()
    }

    pub(super) fn expire(&mut self, now: Instant) -> Option<GrantRequest> {
        let expired = self
            .active
            .as_ref()
            .is_some_and(|prompt| prompt.expires_at() <= now);
        if !expired {
            return None;
        }
        self.active.take().map(|prompt| prompt.into_request())
    }

    pub(crate) fn wait_timeout(&self, now: Instant) -> Option<Duration> {
        let prompt = self.active.as_ref()?;
        Some(prompt.expires_at().saturating_duration_since(now))
    }

    pub(super) fn activate_next(&mut self, now: Instant) -> bool {
        if self.active.is_some() {
            return false;
        }
        let Some(request) = self.pending.pop_front() else {
            return false;
        };
        self.active = Some(GrantPrompt::new(request, now));
        true
    }

    fn remember_startup(&mut self, key: Option<StartupKey>) {
        if let Some(key) = key {
            self.seen_startup.insert(key);
        }
    }
}

pub(super) fn changed_request_summary(request: &GrantRequest) -> String {
    if !request.changed || request.changed_requests.is_empty() {
        return String::new();
    }
    let shown = request
        .changed_requests
        .iter()
        .take(2)
        .map(|line| sanitize(line))
        .collect::<Vec<_>>()
        .join(", ");
    format!(" Changed: {shown}.")
}

pub(super) fn navigation_key(key: KeyEvent) -> Option<isize> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(-1),
        KeyCode::Down | KeyCode::Char('j') => Some(1),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
