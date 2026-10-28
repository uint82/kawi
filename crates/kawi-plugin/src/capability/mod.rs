mod permissions;
mod scope;

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub use permissions::{effective, parse_permissions, PermissionsFile, PluginPermissions};
pub use scope::{intersect_scope, Scope};

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
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityRequests {
    scopes: BTreeMap<Capability, Scope>,
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
        self.scopes.is_empty()
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
        serde_json::to_string(&Value::Object(root)).unwrap_or_default()
    }

    pub fn request_hash(&self) -> String {
        let digest = Sha256::digest(self.canonical_json().as_bytes());
        format!("sha256:{digest:x}")
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
