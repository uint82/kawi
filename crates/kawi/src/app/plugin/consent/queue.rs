use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_plugin::Capability;

use super::super::text::sanitize;

const CALL_CHOICES: [&str; 3] = ["Allow once", "Allow always", "Deny"];
const STARTUP_CHOICES: [&str; 2] = ["Allow always", "Deny"];
const MAX_PENDING: usize = 8;
const PROMPT_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GrantKind {
    Call,
    Startup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GrantChoice {
    Once,
    Always,
    Deny,
}

impl GrantKind {
    pub(super) fn choices(self) -> &'static [&'static str] {
        match self {
            Self::Call => &CALL_CHOICES,
            Self::Startup => &STARTUP_CHOICES,
        }
    }

    fn initial_selection(self) -> usize {
        self.choices().len() - 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GrantRequest {
    pub(super) plugin: String,
    pub(super) capability: Capability,
    pub(super) changed: bool,
    pub(super) changed_requests: Vec<String>,
    pub(super) kind: GrantKind,
}

#[derive(Debug, Clone)]
pub(super) struct GrantPrompt {
    request: GrantRequest,
    selected: usize,
    expires_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Enqueue {
    Activated,
    Queued,
    Duplicate,
    Full,
}

#[derive(Debug)]
pub(crate) struct GrantQueue {
    active: Option<GrantPrompt>,
    pending: VecDeque<GrantRequest>,
    call_choices: [String; 3],
    startup_choices: [String; 2],
}

impl Default for GrantQueue {
    fn default() -> Self {
        Self {
            active: None,
            pending: VecDeque::new(),
            call_choices: CALL_CHOICES.map(str::to_string),
            startup_choices: STARTUP_CHOICES.map(str::to_string),
        }
    }
}

impl GrantQueue {
    pub(super) fn enqueue(&mut self, request: GrantRequest, now: Instant) -> Enqueue {
        if self.is_pending(&request) {
            return Enqueue::Duplicate;
        }
        if self.active.is_none() {
            self.active = Some(GrantPrompt::new(request, now));
            return Enqueue::Activated;
        }
        if self.pending.len() >= MAX_PENDING {
            return Enqueue::Full;
        }
        self.pending.push_back(request);
        Enqueue::Queued
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
        lift(
            self.active.as_ref().map(|prompt| prompt.request.kind),
            &self.call_choices,
            &self.startup_choices,
        )
    }

    pub(super) fn expire(&mut self, now: Instant) -> Option<GrantRequest> {
        let expired = self
            .active
            .as_ref()
            .is_some_and(|prompt| prompt.expires_at <= now);
        if !expired {
            return None;
        }
        self.active.take().map(|prompt| prompt.request)
    }

    pub(crate) fn wait_timeout(&self, now: Instant) -> Option<Duration> {
        let prompt = self.active.as_ref()?;
        Some(prompt.expires_at.saturating_duration_since(now))
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

    fn is_pending(&self, request: &GrantRequest) -> bool {
        self.active
            .as_ref()
            .is_some_and(|prompt| same_request(&prompt.request, request))
            || self
                .pending
                .iter()
                .any(|pending| same_request(pending, request))
    }
}

impl GrantPrompt {
    fn new(request: GrantRequest, now: Instant) -> Self {
        let selected = request.kind.initial_selection();
        Self {
            request,
            selected,
            expires_at: now + PROMPT_TTL,
        }
    }

    pub(super) fn selected(&self) -> usize {
        self.selected
    }

    pub(super) fn request(&self) -> &GrantRequest {
        &self.request
    }

    pub(super) fn move_by(&mut self, delta: isize) {
        let choices = self.request.kind.choices().len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(choices) as usize;
    }

    pub(super) fn choice(&self) -> GrantChoice {
        match (self.request.kind, self.selected) {
            (GrantKind::Call, 0) => GrantChoice::Once,
            (GrantKind::Call, 1) => GrantChoice::Always,
            (GrantKind::Startup, 0) => GrantChoice::Always,
            _ => GrantChoice::Deny,
        }
    }

    pub(super) fn accepts_key(key: KeyEvent) -> bool {
        !key.modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    }
}

pub(super) fn changed_request_summary(request: &GrantRequest) -> String {
    if !request.changed {
        return String::new();
    }
    if request.changed_requests.is_empty() {
        return " Request hash changed; grants were withheld.".to_string();
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

fn same_request(left: &GrantRequest, right: &GrantRequest) -> bool {
    left.plugin == right.plugin && left.capability == right.capability && left.kind == right.kind
}

fn lift<'a>(
    kind: Option<GrantKind>,
    call_choices: &'a [String],
    startup_choices: &'a [String],
) -> &'a [String] {
    match kind {
        Some(GrantKind::Startup) => startup_choices,
        Some(GrantKind::Call) | None => call_choices,
    }
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
