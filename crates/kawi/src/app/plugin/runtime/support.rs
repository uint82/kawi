use std::collections::{BTreeMap, BTreeSet};

use kawi_plugin::{Capability, EffectiveCapabilities, Scope};

pub(super) fn describe_capabilities(
    capabilities: impl Iterator<Item = Capability>,
    sandboxed: bool,
) -> String {
    let names = capabilities
        .map(|capability| describe_capability(capability, sandboxed))
        .collect::<Vec<_>>();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    }
}

fn describe_capability(capability: Capability, sandboxed: bool) -> String {
    if capability == Capability::NetConnect {
        return format!("{} (advisory)", capability.name());
    }
    if capability.is_advisory() && sandboxed {
        return format!("{} (enforced)", capability.name());
    }
    if capability.is_advisory() {
        return format!("{} (advisory)", capability.name());
    }
    capability.name().to_string()
}

pub(super) fn denial_lines(
    plugin: &str,
    capability: Capability,
    scope: Option<Scope>,
) -> [String; 3] {
    [
        format!("denied {}", capability.name()),
        format!("add to permissions.toml: [plugins.{plugin}.grant]"),
        format!(
            "\"{}\" = {}",
            capability.name(),
            grant_value(scope.as_ref())
        ),
    ]
}

fn grant_value(scope: Option<&Scope>) -> String {
    match scope {
        None | Some(Scope::Unscoped) => "true".to_string(),
        Some(Scope::Paths(paths)) => format!("{{ paths = {paths:?} }}"),
        Some(Scope::Commands(commands)) => format!("{{ commands = {commands:?} }}"),
        Some(Scope::Hosts(hosts)) => format!("{{ hosts = {hosts:?} }}"),
    }
}

pub(super) fn changed_sandbox_plugins(
    previous: &BTreeMap<String, EffectiveCapabilities>,
    next: &BTreeMap<String, EffectiveCapabilities>,
) -> BTreeSet<String> {
    let mut changed = BTreeSet::new();
    for name in previous.keys().chain(next.keys()).collect::<BTreeSet<_>>() {
        if sandbox_scope_changed(previous.get(name), next.get(name)) {
            changed.insert(name.clone());
        }
    }
    changed
}

fn sandbox_scope_changed(
    previous: Option<&EffectiveCapabilities>,
    next: Option<&EffectiveCapabilities>,
) -> bool {
    [
        Capability::FsRead,
        Capability::FsWrite,
        Capability::ProcessSpawn,
    ]
    .into_iter()
    .any(|capability| {
        previous.and_then(|effective| effective.scope(capability))
            != next.and_then(|effective| effective.scope(capability))
    })
}

pub(super) fn same_path(left: &std::path::Path, right: &std::path::Path) -> bool {
    left == right
        || match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
            (Ok(left), Ok(right)) => left == right,
            _ => false,
        }
}
