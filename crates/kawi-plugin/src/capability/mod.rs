mod diff;
mod permissions;
mod registry;
mod scope;
mod state;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub use diff::{diff, reconcile_approved, CapabilityChange, ChangeKind};
pub use permissions::{
    effective, parse_permissions, EffectiveCapabilities, PermissionsFile, PluginPermissions,
};
pub use registry::CapabilityRegistry;
#[cfg(feature = "wasm")]
pub(crate) use scope::HostPort;
pub use scope::{intersect_scope, Scope};
pub use state::{load_permission_state, PermissionState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    EditorRead,
    EditorWrite,
    UiMessage,
    DiagnosticsRead,
    DiagnosticsPublish,
    FsRead,
    FsWrite,
    ProcessSpawn,
    NetConnect,
}

impl Capability {
    pub const ALL: [Self; 9] = [
        Self::EditorRead,
        Self::EditorWrite,
        Self::UiMessage,
        Self::DiagnosticsRead,
        Self::DiagnosticsPublish,
        Self::FsRead,
        Self::FsWrite,
        Self::ProcessSpawn,
        Self::NetConnect,
    ];

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "editor.read" => Some(Self::EditorRead),
            "editor.write" => Some(Self::EditorWrite),
            "ui.message" => Some(Self::UiMessage),
            "diagnostics.read" => Some(Self::DiagnosticsRead),
            "diagnostics.publish" => Some(Self::DiagnosticsPublish),
            "fs.read" => Some(Self::FsRead),
            "fs.write" => Some(Self::FsWrite),
            "process.spawn" => Some(Self::ProcessSpawn),
            "net.connect" => Some(Self::NetConnect),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::EditorRead => "editor.read",
            Self::EditorWrite => "editor.write",
            Self::UiMessage => "ui.message",
            Self::DiagnosticsRead => "diagnostics.read",
            Self::DiagnosticsPublish => "diagnostics.publish",
            Self::FsRead => "fs.read",
            Self::FsWrite => "fs.write",
            Self::ProcessSpawn => "process.spawn",
            Self::NetConnect => "net.connect",
        }
    }

    pub fn is_scoped(self) -> bool {
        matches!(
            self,
            Self::FsRead | Self::FsWrite | Self::ProcessSpawn | Self::NetConnect
        )
    }

    pub fn is_default(self) -> bool {
        self == Self::UiMessage
    }

    pub fn is_advisory(self) -> bool {
        matches!(
            self,
            Self::FsRead | Self::FsWrite | Self::ProcessSpawn | Self::NetConnect
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityRequests {
    scopes: BTreeMap<Capability, Scope>,
    commands: BTreeMap<String, BTreeSet<Capability>>,
    artifact: Option<String>,
}

impl CapabilityRequests {
    pub fn get(&self, capability: Capability) -> Option<&Scope> {
        self.scopes.get(&capability)
    }

    pub fn contains(&self, capability: Capability) -> bool {
        self.scopes.contains_key(&capability)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Capability, &Scope)> {
        self.scopes
            .iter()
            .map(|(capability, scope)| (*capability, scope))
    }

    pub fn is_empty(&self) -> bool {
        self.scopes.is_empty() && self.commands.is_empty()
    }

    pub fn command_gate(&self, id: &str) -> Option<&BTreeSet<Capability>> {
        self.commands.get(id)
    }

    pub fn command_gates(&self) -> &BTreeMap<String, BTreeSet<Capability>> {
        &self.commands
    }

    pub fn set_command_gates(&mut self, gates: BTreeMap<String, BTreeSet<Capability>>) {
        self.commands = gates;
    }

    pub fn canonical_json(&self) -> String {
        let capabilities = self
            .scopes
            .iter()
            .map(|(capability, scope)| {
                let value = match scope {
                    Scope::Unscoped => Value::Bool(true),
                    _ => Value::Object(scope.to_json_object()),
                };
                (capability.name().to_string(), value)
            })
            .collect::<BTreeMap<_, _>>();
        let mut root = Map::new();
        root.insert(
            "capabilities".to_string(),
            Value::Object(capabilities.into_iter().collect()),
        );
        if !self.commands.is_empty() {
            let commands = self
                .commands
                .iter()
                .filter(|(_, capabilities)| !capabilities.is_empty())
                .map(|(id, capabilities)| {
                    let names = capabilities
                        .iter()
                        .map(|capability| Value::String(capability.name().to_string()))
                        .collect::<Vec<_>>();
                    (id.clone(), Value::Array(names))
                })
                .collect::<Map<_, _>>();
            root.insert("commands".to_string(), Value::Object(commands));
        }
        if let Some(artifact) = &self.artifact {
            root.insert("component".to_string(), Value::String(artifact.clone()));
        }
        serde_json::to_string(&Value::Object(root)).unwrap_or_default()
    }

    pub fn set_artifact_hash(&mut self, hash: &str) {
        self.artifact = Some(hash.to_string());
    }

    pub fn request_hash(&self) -> String {
        let digest = Sha256::digest(self.canonical_json().as_bytes());
        format!("sha256:{digest:x}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodCapability {
    Unknown,
    None,
    Required(Capability),
}

pub fn host_method_capability(method: &str) -> MethodCapability {
    match method {
        "editor.get_state" => MethodCapability::Required(Capability::EditorRead),
        "editor.apply_edit" => MethodCapability::Required(Capability::EditorWrite),
        "ui.message" => MethodCapability::Required(Capability::UiMessage),
        "editor.set_diagnostics" => MethodCapability::Required(Capability::DiagnosticsPublish),
        _ => MethodCapability::Unknown,
    }
}

pub fn is_host_method_allowed(registry: &CapabilityRegistry, plugin: &str, method: &str) -> bool {
    match host_method_capability(method) {
        MethodCapability::None => true,
        MethodCapability::Required(capability) => registry.has(plugin, capability),
        MethodCapability::Unknown => false,
    }
}

pub fn advertised_host_methods(registry: &CapabilityRegistry, plugin: &str) -> Vec<&'static str> {
    crate::protocol::HOST_METHODS
        .iter()
        .copied()
        .filter(|method| is_host_method_allowed(registry, plugin, method))
        .collect()
}

pub fn is_event_allowed(registry: &CapabilityRegistry, plugin: &str, event: &str) -> bool {
    event_capability(event).is_none_or(|capability| registry.has(plugin, capability))
}

pub fn advertised_events(registry: &CapabilityRegistry, plugin: &str) -> Vec<&'static str> {
    crate::protocol::EVENT_NAMES
        .iter()
        .copied()
        .filter(|event| is_event_allowed(registry, plugin, event))
        .collect()
}

pub fn event_capability(event: &str) -> Option<Capability> {
    match event {
        "buffer_changed" | "buffer_opened" | "file_opened" | "buffer_closed" | "file_saved"
        | "mode_changed" => Some(Capability::EditorRead),
        "diagnostics_changed" => Some(Capability::DiagnosticsRead),
        _ => None,
    }
}

pub fn parse_capability_requests(table: &toml::Table) -> (CapabilityRequests, Vec<String>) {
    let mut requests = CapabilityRequests::default();
    let mut warnings = Vec::new();
    for (name, value) in table {
        let Some(capability) = Capability::parse(name) else {
            warnings.push(format!("unknown capability: {name}"));
            continue;
        };
        match scope::parse_request_scope(capability, value) {
            Ok(scope) => {
                requests.scopes.insert(capability, scope);
            }
            Err(message) => warnings.push(format!("{name}: {message}")),
        }
    }
    (requests, warnings)
}

#[cfg(test)]
mod tests;
