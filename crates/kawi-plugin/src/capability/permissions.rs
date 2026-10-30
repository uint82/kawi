use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use super::diff::{diff, diff_command_gates, CapabilityChange, ChangeKind, CommandGateChange};
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
    #[serde(default)]
    pub command_gates: Option<BTreeMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveCapabilities {
    pub granted: BTreeMap<Capability, Scope>,
    pub approved: BTreeMap<Capability, Scope>,
    pub approved_commands: BTreeMap<String, BTreeSet<Capability>>,
    pub command_gate_changes: Vec<CommandGateChange>,
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
    let approved = resolve_approved(permissions, requests);
    if let Some(warning) = approved.warning {
        result.warnings.push(warning);
    }
    result.approved = approved.capabilities;
    result.approved_commands = approved.commands;
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
    let gate_changes = diff_command_gates(&result.approved_commands, requests);
    result.changed |= !gate_changes.is_empty();
    result.changed_requests.extend(
        gate_changes
            .iter()
            .flat_map(CommandGateChange::labels)
            .collect::<Vec<_>>(),
    );
    result.command_gate_changes = gate_changes.clone();
    if gate_changes.iter().any(CommandGateChange::widens) {
        result
            .warnings
            .push("command gate removed; review widened access".to_string());
    }
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

#[derive(Default)]
struct ApprovedRequests {
    capabilities: BTreeMap<Capability, Scope>,
    commands: BTreeMap<String, BTreeSet<Capability>>,
    warning: Option<String>,
}

fn resolve_approved(
    permissions: Option<&PluginPermissions>,
    requests: &CapabilityRequests,
) -> ApprovedRequests {
    let Some(permissions) = permissions else {
        return ApprovedRequests::default();
    };
    if let Some(approved) = &permissions.approved {
        let capabilities = match parse_approved(approved) {
            Ok(approved) => approved,
            Err(error) => {
                return ApprovedRequests {
                    warning: Some(format!(
                        "approved request record is corrupt: {error}; every request is new"
                    )),
                    ..ApprovedRequests::default()
                };
            }
        };
        let commands = match parse_approved_commands(permissions.command_gates.as_ref()) {
            Ok(commands) => commands,
            Err(error) => {
                return ApprovedRequests {
                    capabilities,
                    warning: Some(format!("approved command gates are corrupt: {error}")),
                    ..ApprovedRequests::default()
                };
            }
        };
        return ApprovedRequests {
            capabilities,
            commands,
            warning: None,
        };
    }
    if permissions
        .request_hash
        .as_ref()
        .is_some_and(|hash| hash == &requests.request_hash())
    {
        return ApprovedRequests {
            capabilities: requests
                .iter()
                .map(|(capability, scope)| (capability, scope.clone()))
                .collect(),
            commands: requests.command_gates().clone(),
            warning: None,
        };
    }
    ApprovedRequests::default()
}

fn parse_approved_commands(
    commands: Option<&BTreeMap<String, Vec<String>>>,
) -> Result<BTreeMap<String, BTreeSet<Capability>>, String> {
    let mut parsed = BTreeMap::new();
    for (command, names) in commands.into_iter().flatten() {
        let mut capabilities = BTreeSet::new();
        for name in names {
            let capability =
                Capability::parse(name).ok_or_else(|| format!("unknown capability: {name}"))?;
            capabilities.insert(capability);
        }
        if !capabilities.is_empty() {
            parsed.insert(command.clone(), capabilities);
        }
    }
    Ok(parsed)
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
