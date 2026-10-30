use std::time::Instant;

use crossterm::event::{KeyEvent, KeyModifiers};

use super::request::{GrantKind, GrantRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GrantChoice {
    Once,
    Always,
    Deny,
}

#[derive(Debug, Clone)]
pub(super) struct GrantPrompt {
    request: GrantRequest,
    selected: usize,
    expires_at: Instant,
    labels: Vec<String>,
}

impl GrantPrompt {
    pub(super) fn new(request: GrantRequest, now: Instant) -> Self {
        let selected = if request.items.len() == 1 {
            request.kind.initial_selection()
        } else {
            0
        };
        let mut prompt = Self {
            request,
            selected,
            expires_at: now + super::queue::PROMPT_TTL,
            labels: Vec::new(),
        };
        prompt.refresh_labels();
        prompt
    }

    pub(super) fn selected(&self) -> usize {
        self.selected
    }

    pub(super) fn request(&self) -> &GrantRequest {
        &self.request
    }

    pub(super) fn into_request(self) -> GrantRequest {
        self.request
    }

    pub(super) fn expires_at(&self) -> Instant {
        self.expires_at
    }

    pub(super) fn labels(&self) -> &[String] {
        &self.labels
    }

    pub(super) fn is_batch(&self) -> bool {
        self.request.items.len() > 1
    }

    pub(super) fn merge(&mut self, request: GrantRequest) -> bool {
        let added = self.request.merge(&request);
        if added {
            if self.selected >= self.item_count() {
                self.selected = 0;
            }
            self.refresh_labels();
        }
        added
    }

    pub(super) fn move_by(&mut self, delta: isize) {
        let count = self.item_count().max(1) as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(count) as usize;
    }

    pub(super) fn toggle_selected(&mut self) {
        if !self.is_batch() {
            return;
        }
        let kind = self.request.kind;
        if let Some(item) = self.request.items.get_mut(self.selected) {
            item.toggle(kind);
        }
        self.refresh_labels();
    }

    pub(super) fn allow_all(&mut self) {
        self.set_all(GrantChoice::Always);
    }

    pub(super) fn deny_all(&mut self) {
        self.set_all(GrantChoice::Deny);
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

    fn item_count(&self) -> usize {
        if self.is_batch() {
            self.request.items.len()
        } else {
            self.request.kind.choices().len()
        }
    }

    fn set_all(&mut self, choice: GrantChoice) {
        for item in &mut self.request.items {
            item.choice = choice;
        }
        self.refresh_labels();
    }

    fn refresh_labels(&mut self) {
        self.labels = if self.is_batch() {
            let kind = self.request.kind;
            self.request
                .items
                .iter()
                .map(|item| item.label(kind))
                .collect()
        } else {
            self.request
                .kind
                .choices()
                .iter()
                .map(ToString::to_string)
                .collect()
        };
    }
}
