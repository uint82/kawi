use std::collections::BTreeMap;

use serde::Deserialize;

use super::diff::{diff, CapabilityChange, ChangeKind};
use super::scope::{intersect_scope, parse_request_scope, Scope};
use super::{Capability, CapabilityRequests};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionsFile {
    #[serde(default)]
    pub plugins: BTreeMap<String, PluginPermissions>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginPermissions {
    #[serde(default)]
    pub grant: BTreeMap<String, toml::Value>,
    #[serde(default)]
    pub approved: Option<BTreeMap<String, toml::Value>>,
    #[serde(default)]
    pub request_hash: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveCapabilities {
    pub granted: BTreeMap<Capability, Scope>,
    pub approved: BTreeMap<Capability, Scope>,
    pub missing: Vec<Capability>,
    pub stale: Vec<Capability>,
    pub warnings: Vec<String>,
    pub changed: bool,
    pub changed_requests: Vec<String>,
    pub changed_capabilities: Vec<Capability>,
    pub changes: Vec<CapabilityChange>,
}

impl EffectiveCapabilities {
    pub fn has(&self, capability: Capability) -> bool {
        self.granted.contains_key(&capability)
    }

    pub fn scope(&self, capability: Capability) -> Option<&Scope> {
        self.granted.get(&capability)
    }
}

pub fn parse_permissions(text: &str) -> Result<PermissionsFile, String> {
    toml::from_str(text).map_err(|error| error.to_string())
}

pub fn scope_is_partial(requested: &Scope, granted: &Scope) -> bool {
    match (requested, granted) {
        (Scope::Paths(requested), Scope::Paths(granted))
        | (Scope::Commands(requested), Scope::Commands(granted))
        | (Scope::Hosts(requested), Scope::Hosts(granted)) => granted.len() < requested.len(),
        _ => false,
    }
}

pub fn effective(
    requests: &CapabilityRequests,
    permissions: Option<&PluginPermissions>,
) -> EffectiveCapabilities {
    let mut result = EffectiveCapabilities::default();
    let grants = parse_grants(permissions, &mut result);
    let (approved, approved_warning) = resolve_approved(permissions, requests);
    if let Some(warning) = approved_warning {
        result.warnings.push(warning);
    }
    result.approved = approved;
    let changes = diff(&result.approved, requests);
    result.changed = changes
        .iter()
        .any(|change| change.kind != ChangeKind::Unchanged);
    result.changed_requests = changes
        .iter()
        .filter(|change| !change.capability.is_default() && change.kind != ChangeKind::Unchanged)
        .map(|change| change.label())
        .collect();
    result.changed_capabilities = changes
        .iter()
        .filter(|change| change.kind.requires_approval() && !change.capability.is_default())
        .map(|change| change.capability)
        .collect();
    result.changes = changes;
    if !result.changed_capabilities.is_empty() {
        result
            .warnings
            .push("request set changed; added or widened grants withheld".to_string());
    }
    let revoked = revoked_capabilities(permissions, &mut result);
    let changed = result.changed_capabilities.clone();
    let kinds = result
        .changes
        .iter()
        .map(|change| (change.capability, change.kind))
        .collect::<BTreeMap<_, _>>();
    apply_grants(requests, &grants, &revoked, &changed, &kinds, &mut result);
    collect_stale(requests, &grants, &revoked, &mut result);
    result.missing.sort_by_key(|capability| capability.name());
    result.stale.sort_by_key(|capability| capability.name());
    result
}

fn parse_grants(
    permissions: Option<&PluginPermissions>,
    result: &mut EffectiveCapabilities,
) -> BTreeMap<Capability, Scope> {
    let mut grants = BTreeMap::new();
    let Some(permissions) = permissions else {
        return grants;
    };
    for (name, value) in &permissions.grant {
        if value == &toml::Value::Boolean(false) {
            continue;
        }
        let Some(capability) = Capability::parse(name) else {
            result
                .warnings
                .push(format!("unknown grant capability: {name}"));
            continue;
        };
        match parse_request_scope(capability, value) {
            Ok(scope) => {
                grants.insert(capability, scope);
            }
            Err(message) => result.warnings.push(format!("{name}: {message}")),
        }
    }
    grants
}

fn revoked_capabilities(
    permissions: Option<&PluginPermissions>,
    result: &mut EffectiveCapabilities,
) -> Vec<Capability> {
    let mut revoked = Vec::new();
    let Some(permissions) = permissions else {
        return revoked;
    };
    for (name, value) in &permissions.grant {
        if value != &toml::Value::Boolean(false) {
            continue;
        }
        let Some(capability) = Capability::parse(name) else {
            result
                .warnings
                .push(format!("unknown grant capability: {name}"));
            continue;
        };
        if capability.is_default() {
            revoked.push(capability);
        } else {
            result
                .warnings
                .push(format!("{name}: only default capabilities may be revoked"));
        }
    }
    revoked
}

fn resolve_approved(
    permissions: Option<&PluginPermissions>,
    requests: &CapabilityRequests,
) -> (BTreeMap<Capability, Scope>, Option<String>) {
    let Some(permissions) = permissions else {
        return (BTreeMap::new(), None);
    };
    if let Some(approved) = &permissions.approved {
        return match parse_approved(approved) {
            Ok(approved) => (approved, None),
            Err(error) => (
                BTreeMap::new(),
                Some(format!(
                    "approved request record is corrupt: {error}; every request is new"
                )),
            ),
        };
    }
    if permissions
        .request_hash
        .as_ref()
        .is_some_and(|hash| hash == &requests.request_hash())
    {
        return (
            requests
                .iter()
                .map(|(capability, scope)| (capability, scope.clone()))
                .collect(),
            None,
        );
    }
    (BTreeMap::new(), None)
}

fn parse_approved(
    approved: &BTreeMap<String, toml::Value>,
) -> Result<BTreeMap<Capability, Scope>, String> {
    let mut parsed = BTreeMap::new();
    for (name, value) in approved {
        let capability =
            Capability::parse(name).ok_or_else(|| format!("unknown capability: {name}"))?;
        let scope =
            parse_request_scope(capability, value).map_err(|error| format!("{name}: {error}"))?;
        parsed.insert(capability, scope);
    }
    Ok(parsed)
}

fn apply_grants(
    requests: &CapabilityRequests,
    grants: &BTreeMap<Capability, Scope>,
    revoked: &[Capability],
    changed: &[Capability],
    kinds: &BTreeMap<Capability, ChangeKind>,
    result: &mut EffectiveCapabilities,
) {
    for (capability, requested) in requests.iter() {
        if capability.is_default() {
            if revoked.contains(&capability) {
                result.missing.push(capability);
            } else {
                result.granted.insert(capability, Scope::Unscoped);
            }
            continue;
        }
        if changed.contains(&capability) {
            result.missing.push(capability);
            continue;
        }
        let Some(granted) = grants.get(&capability) else {
            result.missing.push(capability);
            continue;
        };
        match effective_scope(requested, granted, kinds.get(&capability).copied()) {
            Some(scope) => {
                if scope_is_partial(requested, &scope) {
                    result
                        .warnings
                        .push(format!("{}: request partially granted", capability.name()));
                }
                result.granted.insert(capability, scope);
            }
            None => result.missing.push(capability),
        }
    }
}

fn effective_scope(requested: &Scope, granted: &Scope, kind: Option<ChangeKind>) -> Option<Scope> {
    intersect_scope(requested, granted).or_else(|| {
        (kind == Some(ChangeKind::Narrowed) && granted.covers(requested)).then(|| requested.clone())
    })
}

fn collect_stale(
    requests: &CapabilityRequests,
    grants: &BTreeMap<Capability, Scope>,
    revoked: &[Capability],
    result: &mut EffectiveCapabilities,
) {
    for capability in grants.keys() {
        if !requests.contains(*capability) {
            result.stale.push(*capability);
        }
    }
    for capability in revoked {
        if !requests.contains(*capability) {
            result.stale.push(*capability);
        }
    }
}

#[cfg(test)]
mod tests;
