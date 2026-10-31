use std::collections::{BTreeMap, BTreeSet};

use super::{Capability, CapabilityRequests, Scope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Widened,
    Narrowed,
    Removed,
    Unchanged,
}

impl ChangeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Widened => "widened",
            Self::Narrowed => "narrowed",
            Self::Removed => "removed",
            Self::Unchanged => "unchanged",
        }
    }

    pub fn requires_approval(self) -> bool {
        matches!(self, Self::Added | Self::Widened)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityChange {
    pub capability: Capability,
    pub kind: ChangeKind,
}

impl CapabilityChange {
    pub fn label(&self) -> String {
        format!("{} {}", self.capability.name(), self.kind.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandGateChange {
    pub command: String,
    pub added: BTreeSet<Capability>,
    pub removed: BTreeSet<Capability>,
}

impl CommandGateChange {
    pub fn labels(&self) -> Vec<String> {
        let mut labels = Vec::new();
        if !self.added.is_empty() {
            labels.push(format!(
                "command {} requires {}",
                self.command,
                self.added
                    .iter()
                    .map(|capability| capability.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !self.removed.is_empty() {
            labels.push(format!(
                "command {} no longer requires {}",
                self.command,
                self.removed
                    .iter()
                    .map(|capability| capability.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        labels
    }

    pub fn widens(&self) -> bool {
        !self.removed.is_empty()
    }
}

pub fn diff_command_gates(
    approved: &BTreeMap<String, BTreeSet<Capability>>,
    requests: &CapabilityRequests,
) -> Vec<CommandGateChange> {
    let commands = approved
        .keys()
        .chain(requests.command_gates().keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    commands
        .into_iter()
        .filter_map(|command| {
            let previous = approved.get(&command).cloned().unwrap_or_default();
            let next = requests.command_gate(&command).cloned().unwrap_or_default();
            let added = next
                .difference(&previous)
                .copied()
                .collect::<BTreeSet<Capability>>();
            let removed = previous
                .difference(&next)
                .copied()
                .collect::<BTreeSet<Capability>>();
            if added.is_empty() && removed.is_empty() {
                return None;
            }
            Some(CommandGateChange {
                command,
                added,
                removed,
            })
        })
        .collect()
}

pub fn diff(
    approved: &BTreeMap<Capability, Scope>,
    requests: &CapabilityRequests,
) -> Vec<CapabilityChange> {
    let capabilities = approved
        .keys()
        .copied()
        .chain(requests.iter().map(|(capability, _)| capability))
        .collect::<BTreeSet<_>>();
    capabilities
        .into_iter()
        .filter_map(|capability| {
            let previous = approved.get(&capability);
            let next = requests.get(capability);
            let kind = match (previous, next) {
                (None, Some(_)) => ChangeKind::Added,
                (Some(_), None) => ChangeKind::Removed,
                (Some(previous), Some(next)) => scope_change(previous, next),
                (None, None) => return None,
            };
            Some(CapabilityChange { capability, kind })
        })
        .collect()
}

fn scope_change(previous: &Scope, next: &Scope) -> ChangeKind {
    if previous == next {
        return ChangeKind::Unchanged;
    }
    if next.covers(previous) && previous.covers(next) {
        return ChangeKind::Unchanged;
    }
    if next.covers(previous) {
        return ChangeKind::Widened;
    }
    if previous.covers(next) {
        return ChangeKind::Narrowed;
    }
    ChangeKind::Widened
}

pub fn reconcile_approved(
    approved: &BTreeMap<Capability, Scope>,
    requests: &CapabilityRequests,
    decisions: &BTreeMap<Capability, bool>,
) -> BTreeMap<Capability, Scope> {
    let changes = diff(approved, requests)
        .into_iter()
        .map(|change| (change.capability, change.kind))
        .collect::<BTreeMap<_, _>>();
    let mut next = BTreeMap::new();
    for (capability, scope) in requests.iter() {
        let kind = changes
            .get(&capability)
            .copied()
            .unwrap_or(ChangeKind::Unchanged);
        if capability.is_default()
            || !kind.requires_approval()
            || decisions.get(&capability) == Some(&true)
        {
            next.insert(capability, scope.clone());
        } else if let Some(previous) = approved.get(&capability) {
            next.insert(capability, previous.clone());
        }
    }
    next
}
