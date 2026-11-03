use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use super::{Capability, CapabilityRegistry, Scope};
use crate::protocol::RpcError;

pub(crate) fn authorize_command(
    plugin: &str,
    capabilities: &CapabilityRegistry,
    command: &str,
    resolve: impl Fn(&str) -> Option<PathBuf>,
    not_found: impl FnOnce(&str) -> RpcError,
) -> Result<PathBuf, RpcError> {
    if !capabilities.has(plugin, Capability::ProcessSpawn) {
        return Err(denied(command));
    }
    let requested = resolve(command).ok_or_else(|| not_found(command))?;
    let Some(Scope::Commands(commands)) = capabilities.scope(plugin, Capability::ProcessSpawn)
    else {
        return Err(denied(command));
    };
    if commands
        .iter()
        .any(|granted| resolve(granted).is_some_and(|granted| granted == requested))
    {
        Ok(requested)
    } else {
        Err(denied(command))
    }
}

fn denied(command: &str) -> RpcError {
    RpcError::capability_denied(
        Capability::ProcessSpawn.name(),
        Some(serde_json::json!({ "command": command })),
    )
}

pub(crate) fn validate(command: &str) -> Result<(), String> {
    if command.trim().is_empty() {
        return Err("commands must not be empty".to_string());
    }
    if is_supported(command) {
        Ok(())
    } else {
        Err("commands must be names or absolute paths".to_string())
    }
}

pub(crate) fn resolve(command: &str) -> Option<PathBuf> {
    if !is_supported(command) {
        return None;
    }
    let path = kawi_process::find_executable(command)?;
    std::fs::canonicalize(path).ok()
}

pub(super) fn intersect(requested: &[String], granted: &[String]) -> Option<Vec<String>> {
    let granted = granted
        .iter()
        .filter_map(|command| resolve(command))
        .collect::<BTreeSet<_>>();
    let values = requested
        .iter()
        .filter_map(|command| resolve(command))
        .filter(|command| granted.contains(command))
        .map(|command| command.to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>();
    (!values.is_empty()).then(|| values.into_iter().collect())
}

pub(super) fn covers(outer: &[String], inner: &[String]) -> bool {
    let Some(inner) = canonical_set(inner) else {
        return false;
    };
    let outer = outer
        .iter()
        .filter_map(|command| resolve(command))
        .collect::<BTreeSet<_>>();
    inner.is_subset(&outer)
}

fn canonical_set(commands: &[String]) -> Option<BTreeSet<PathBuf>> {
    commands.iter().map(|command| resolve(command)).collect()
}

fn is_supported(command: &str) -> bool {
    let path = Path::new(command);
    path.is_absolute() || is_bare_name(path)
}

fn is_bare_name(path: &Path) -> bool {
    let mut components = path.components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

#[cfg(test)]
mod tests;
