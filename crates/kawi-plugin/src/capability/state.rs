use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::manifest::PluginCatalog;

use super::{effective, parse_permissions, EffectiveCapabilities};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionState {
    pub grants: BTreeMap<String, EffectiveCapabilities>,
    pub warnings: Vec<String>,
    pub file_error: Option<String>,
}

pub fn load_permission_state(catalog: &PluginCatalog, path: &Path) -> PermissionState {
    let mut state = PermissionState::default();
    let permissions = match fs::read_to_string(path) {
        Ok(text) => match parse_permissions(&text) {
            Ok(permissions) => Some(permissions),
            Err(error) => {
                let message = format!("{}: {error}", path.display());
                state.file_error = Some(message.clone());
                state.warnings.push(message);
                None
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            let message = format!("{}: cannot read permissions: {error}", path.display());
            state.file_error = Some(message.clone());
            state.warnings.push(message);
            None
        }
    };
    for plugin in &catalog.plugins {
        let name = plugin.manifest.name.as_str();
        let entry = permissions
            .as_ref()
            .and_then(|permissions| permissions.plugins.get(name));
        let effective = effective(&plugin.manifest.capability_requests, entry);
        for warning in &effective.warnings {
            state.warnings.push(format!("{name}: {warning}"));
        }
        for capability in &effective.missing {
            state.warnings.push(format!(
                "{name}: capability not granted: {}",
                capability.name()
            ));
        }
        for capability in &effective.stale {
            state
                .warnings
                .push(format!("{name}: stale grant: {}", capability.name()));
        }
        state.grants.insert(name.to_string(), effective);
    }
    if let Some(permissions) = &permissions {
        for name in permissions.plugins.keys() {
            if !catalog
                .plugins
                .iter()
                .any(|plugin| plugin.manifest.name == *name)
            {
                state
                    .warnings
                    .push(format!("unknown plugin in permissions: {name}"));
            }
        }
    }
    state.warnings.sort();
    state.warnings.dedup();
    state
}

#[cfg(test)]
mod tests;
