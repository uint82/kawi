use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_plugin::Capability;

use super::super::text::sanitize;

pub(super) const CHOICES: [&str; 3] = ["Allow once", "Allow always", "Deny"];
const MAX_PENDING: usize = 8;
const PROMPT_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GrantRequest {
    pub(super) plugin: String,
    pub(super) capability: Capability,
    pub(super) changed: bool,
    pub(super) changed_requests: Vec<String>,
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
    choices: [String; 3],
}

impl Default for GrantQueue {
    fn default() -> Self {
        Self {
            active: None,
            pending: VecDeque::new(),
            choices: CHOICES.map(str::to_string),
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
        &self.choices
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
        Self {
            request,
            selected: CHOICES.len() - 1,
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
        self.selected =
            (self.selected as isize + delta).rem_euclid(CHOICES.len() as isize) as usize;
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
    left.plugin == right.plugin && left.capability == right.capability
}

pub(super) fn navigation_key(key: KeyEvent) -> Option<isize> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(-1),
        KeyCode::Down | KeyCode::Char('j') => Some(1),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(plugin: &str, capability: Capability) -> GrantRequest {
        GrantRequest {
            plugin: plugin.to_string(),
            capability,
            changed: false,
            changed_requests: Vec::new(),
        }
    }

    #[test]
    fn queues_one_prompt_and_deduplicates_requests() {
        let now = Instant::now();
        let mut queue = GrantQueue::default();
        assert_eq!(
            queue.enqueue(request("a", Capability::EditorRead), now),
            Enqueue::Activated
        );
        assert_eq!(
            queue.enqueue(request("b", Capability::EditorRead), now),
            Enqueue::Queued
        );
        assert_eq!(
            queue.enqueue(request("a", Capability::EditorRead), now),
            Enqueue::Duplicate
        );
    }

    #[test]
    fn caps_the_pending_queue() {
        let now = Instant::now();
        let mut queue = GrantQueue::default();
        queue.enqueue(request("active", Capability::EditorRead), now);
        for index in 0..MAX_PENDING {
            assert_eq!(
                queue.enqueue(
                    request(&format!("pending-{index}"), Capability::EditorRead),
                    now
                ),
                Enqueue::Queued
            );
        }
        assert_eq!(
            queue.enqueue(request("overflow", Capability::EditorRead), now),
            Enqueue::Full
        );
    }

    #[test]
    fn expiry_lapses_the_active_prompt() {
        let now = Instant::now();
        let mut queue = GrantQueue::default();
        queue.enqueue(request("a", Capability::EditorRead), now);
        assert!(queue.expire(now + PROMPT_TTL).is_some());
        assert!(queue.active().is_none());
    }

    #[test]
    fn changed_summary_is_bounded() {
        let mut request = request("a", Capability::FsWrite);
        request.changed = true;
        request.changed_requests = (0..5).map(|index| format!("cap {index} added")).collect();
        assert_eq!(
            changed_request_summary(&request),
            " Changed: cap 0 added, cap 1 added."
        );
    }
}
