use std::collections::BTreeSet;

use kawi_plugin::{Capability, ChangeKind};

use super::prompt::GrantChoice;

const CALL_CHOICES: [&str; 3] = ["Allow once", "Allow always", "Deny"];
const STARTUP_CHOICES: [&str; 2] = ["Allow always", "Deny"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GrantKind {
    Call,
    Startup,
}

impl GrantKind {
    pub(super) fn choices(self) -> &'static [&'static str] {
        match self {
            Self::Call => &CALL_CHOICES,
            Self::Startup => &STARTUP_CHOICES,
        }
    }

    pub(super) fn initial_selection(self) -> usize {
        self.choices().len() - 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GrantItem {
    pub(super) capability: Capability,
    pub(super) change: Option<ChangeKind>,
    pub(super) choice: GrantChoice,
}

impl GrantItem {
    pub(super) fn new(
        capability: Capability,
        change: Option<ChangeKind>,
        _kind: GrantKind,
    ) -> Self {
        Self {
            capability,
            change,
            choice: GrantChoice::Deny,
        }
    }

    pub(super) fn label(&self, kind: GrantKind) -> String {
        let change = self.change.map(ChangeKind::label).unwrap_or("requested");
        format!(
            "{}: {} ({})",
            self.capability.name(),
            choice_label(kind, self.choice),
            change
        )
    }

    pub(super) fn toggle(&mut self, kind: GrantKind) {
        self.choice = match (kind, self.choice) {
            (GrantKind::Call, GrantChoice::Deny) => GrantChoice::Once,
            (GrantKind::Call, GrantChoice::Once) => GrantChoice::Always,
            (GrantKind::Call, GrantChoice::Always) => GrantChoice::Deny,
            (GrantKind::Startup, GrantChoice::Deny) => GrantChoice::Always,
            (GrantKind::Startup, _) => GrantChoice::Deny,
        };
    }
}

fn choice_label(kind: GrantKind, choice: GrantChoice) -> &'static str {
    match (kind, choice) {
        (GrantKind::Call, GrantChoice::Once) => "allow once",
        (GrantKind::Call, GrantChoice::Always) | (GrantKind::Startup, GrantChoice::Always) => {
            "allow always"
        }
        (GrantKind::Startup, GrantChoice::Once) => "allow once",
        (_, GrantChoice::Deny) => "deny",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GrantRequest {
    pub(super) plugin: String,
    pub(super) changed: bool,
    pub(super) changed_requests: Vec<String>,
    pub(super) kind: GrantKind,
    pub(super) request_hash: String,
    pub(super) items: Vec<GrantItem>,
}

impl GrantRequest {
    pub(super) fn capability(&self) -> Capability {
        self.items[0].capability
    }

    pub(super) fn same_batch(&self, other: &Self) -> bool {
        self.plugin == other.plugin
            && self.kind == other.kind
            && self.request_hash == other.request_hash
    }

    pub(super) fn merge(&mut self, other: &Self) -> bool {
        let mut added = false;
        for item in &other.items {
            if self
                .items
                .iter()
                .any(|existing| existing.capability == item.capability)
            {
                continue;
            }
            self.items.push(item.clone());
            added = true;
        }
        self.changed |= other.changed;
        for label in &other.changed_requests {
            if !self.changed_requests.contains(label) {
                self.changed_requests.push(label.clone());
            }
        }
        added
    }

    pub(super) fn startup_key(&self) -> Option<StartupKey> {
        (self.kind == GrantKind::Startup).then(|| StartupKey {
            plugin: self.plugin.clone(),
            request_hash: self.request_hash.clone(),
            capabilities: self
                .items
                .iter()
                .map(|item| item.capability.name())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(","),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct StartupKey {
    plugin: String,
    request_hash: String,
    capabilities: String,
}
