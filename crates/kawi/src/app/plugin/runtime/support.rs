use std::collections::{BTreeMap, BTreeSet};

use kawi_plugin::{Capability, EffectiveCapabilities, PluginCatalog, PluginRuntime, Scope};

use crate::permission_lines::grant_lines;
use crate::plugin_labels::WASM_NETWORK_ENFORCEMENT;

pub(super) fn describe_capabilities(
    capabilities: impl Iterator<Item = Capability>,
    wasm: bool,
    sandboxed: bool,
) -> String {
    let names = capabilities
        .map(|capability| describe_capability(capability, wasm, sandboxed))
        .collect::<Vec<_>>();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    }
}

fn describe_capability(capability: Capability, wasm: bool, sandboxed: bool) -> String {
    if capability == Capability::NetConnect {
        let enforcement = if wasm {
            WASM_NETWORK_ENFORCEMENT
        } else {
            "advisory"
        };
        return format!("{} ({enforcement})", capability.name());
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
    let [hint, assignment] = grant_lines(plugin, capability, scope.as_ref());
    [format!("denied {}", capability.name()), hint, assignment]
}

pub(super) fn changed_restart_plugins(
    catalog: &PluginCatalog,
    previous: &BTreeMap<String, EffectiveCapabilities>,
    next: &BTreeMap<String, EffectiveCapabilities>,
) -> BTreeSet<String> {
    catalog
        .plugins
        .iter()
        .filter(|plugin| {
            restart_required(
                plugin.manifest.runtime,
                previous.get(&plugin.manifest.name),
                next.get(&plugin.manifest.name),
            )
        })
        .map(|plugin| plugin.manifest.name.clone())
        .collect()
}

fn restart_required(
    runtime: PluginRuntime,
    previous: Option<&EffectiveCapabilities>,
    next: Option<&EffectiveCapabilities>,
) -> bool {
    match runtime {
        PluginRuntime::Process => sandbox_scope_changed(previous, next),
        PluginRuntime::Wasm => granted_capabilities_changed(previous, next),
    }
}

fn granted_capabilities_changed(
    previous: Option<&EffectiveCapabilities>,
    next: Option<&EffectiveCapabilities>,
) -> bool {
    previous.map(|effective| &effective.granted) != next.map(|effective| &effective.granted)
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
