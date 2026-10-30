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
