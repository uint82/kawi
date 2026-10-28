use std::collections::BTreeMap;

use serde::Deserialize;

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
    pub request_hash: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveCapabilities {
    pub granted: BTreeMap<Capability, Scope>,
    pub missing: Vec<Capability>,
    pub stale: Vec<Capability>,
    pub warnings: Vec<String>,
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

pub fn effective(
    requests: &CapabilityRequests,
    permissions: Option<&PluginPermissions>,
) -> EffectiveCapabilities {
    let mut result = EffectiveCapabilities::default();
    let mut grants = BTreeMap::new();
    let mut revoked = Vec::new();
    if let Some(permissions) = permissions {
        for (name, value) in &permissions.grant {
            let Some(capability) = Capability::parse(name) else {
                result
                    .warnings
                    .push(format!("unknown grant capability: {name}"));
                continue;
            };
            if value == &toml::Value::Boolean(false) {
                if capability.is_default() {
                    revoked.push(capability);
                } else {
                    result
                        .warnings
                        .push(format!("{name}: only default capabilities may be revoked"));
                }
                continue;
            }
            match parse_request_scope(capability, value) {
                Ok(scope) => {
                    grants.insert(capability, scope);
                }
                Err(message) => result.warnings.push(format!("{name}: {message}")),
            }
        }
    }
    for (capability, requested) in requests.iter() {
        let granted = grants.get(&capability);
        if granted.is_none() && capability.is_default() && !revoked.contains(&capability) {
            result.granted.insert(capability, Scope::Unscoped);
            continue;
        }
        let Some(granted) = granted else {
            result.missing.push(capability);
            continue;
        };
        match intersect_scope(requested, granted) {
            Some(scope) => {
                result.granted.insert(capability, scope);
            }
            None => result.missing.push(capability),
        }
    }
    for capability in grants.keys() {
        if !requests.contains(*capability) {
            result.stale.push(*capability);
        }
    }
    for capability in revoked {
        if !requests.contains(capability) {
            result.stale.push(capability);
        }
    }
    result.missing.sort_by_key(|capability| capability.name());
    result.stale.sort_by_key(|capability| capability.name());
    result
}

#[cfg(test)]
mod tests;
