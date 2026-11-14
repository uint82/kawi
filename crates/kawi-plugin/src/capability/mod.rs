mod command;
mod method;
mod registry;
mod scope;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub(crate) use command::{authorize_command, resolve as resolve_command};
pub use method::{
    advertised_events, advertised_host_methods, event_capability, event_minor,
    host_method_capability, host_method_minor, is_event_allowed, is_host_method_allowed,
    is_process_event, is_process_host_method, MethodCapability,
};
pub use registry::CapabilityRegistry;
#[cfg(feature = "wasm")]
pub(crate) use scope::HostPort;
pub use scope::{intersect_scope, Scope};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    EditorRead,
    EditorWrite,
    EditorBuffers,
    EditorBuffersWrite,
    EditorWindows,
    EditorControl,
    EditorExecute,
    EditorDecorations,
    UiMessage,
    UiPrompt,
    UiProgress,
    UiStatusline,
    UiPanel,
    DiagnosticsRead,
    DiagnosticsPublish,
    OptionsRead,
    FsRead,
    FsWrite,
    ProcessSpawn,
    NetConnect,
}

impl Capability {
    pub const ALL: [Self; 20] = [
        Self::EditorRead,
        Self::EditorWrite,
        Self::EditorBuffers,
        Self::EditorBuffersWrite,
        Self::EditorWindows,
        Self::EditorControl,
        Self::EditorExecute,
        Self::EditorDecorations,
        Self::UiMessage,
        Self::UiPrompt,
        Self::UiProgress,
        Self::UiStatusline,
        Self::UiPanel,
        Self::DiagnosticsRead,
        Self::DiagnosticsPublish,
        Self::OptionsRead,
        Self::FsRead,
        Self::FsWrite,
        Self::ProcessSpawn,
        Self::NetConnect,
    ];

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "editor.read" => Some(Self::EditorRead),
            "editor.write" => Some(Self::EditorWrite),
            "editor.buffers" => Some(Self::EditorBuffers),
            "editor.buffers.write" => Some(Self::EditorBuffersWrite),
            "editor.windows" => Some(Self::EditorWindows),
            "editor.control" => Some(Self::EditorControl),
            "editor.execute" => Some(Self::EditorExecute),
            "editor.decorations" => Some(Self::EditorDecorations),
            "ui.message" => Some(Self::UiMessage),
            "ui.prompt" => Some(Self::UiPrompt),
            "ui.progress" => Some(Self::UiProgress),
            "ui.statusline" => Some(Self::UiStatusline),
            "ui.panel" => Some(Self::UiPanel),
            "diagnostics.read" => Some(Self::DiagnosticsRead),
            "diagnostics.publish" => Some(Self::DiagnosticsPublish),
            "options.read" => Some(Self::OptionsRead),
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
            Self::EditorBuffers => "editor.buffers",
            Self::EditorBuffersWrite => "editor.buffers.write",
            Self::EditorWindows => "editor.windows",
            Self::EditorControl => "editor.control",
            Self::EditorExecute => "editor.execute",
            Self::EditorDecorations => "editor.decorations",
            Self::UiMessage => "ui.message",
            Self::UiPrompt => "ui.prompt",
            Self::UiProgress => "ui.progress",
            Self::UiStatusline => "ui.statusline",
            Self::UiPanel => "ui.panel",
            Self::DiagnosticsRead => "diagnostics.read",
            Self::DiagnosticsPublish => "diagnostics.publish",
            Self::OptionsRead => "options.read",
            Self::FsRead => "fs.read",
            Self::FsWrite => "fs.write",
            Self::ProcessSpawn => "process.spawn",
            Self::NetConnect => "net.connect",
        }
    }

    pub fn is_scoped(self) -> bool {
        matches!(
            self,
            Self::EditorBuffersWrite
                | Self::EditorExecute
                | Self::FsRead
                | Self::FsWrite
                | Self::ProcessSpawn
                | Self::NetConnect
        )
    }

    pub fn is_advisory(self) -> bool {
        matches!(
            self,
            Self::FsRead | Self::FsWrite | Self::ProcessSpawn | Self::NetConnect
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveCapabilities {
    pub granted: BTreeMap<Capability, Scope>,
}

impl EffectiveCapabilities {
    pub fn has(&self, capability: Capability) -> bool {
        self.granted.contains_key(&capability)
    }

    pub fn scope(&self, capability: Capability) -> Option<&Scope> {
        self.granted.get(&capability)
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

    pub fn granted(&self) -> EffectiveCapabilities {
        EffectiveCapabilities {
            granted: self.scopes.clone(),
        }
    }

    pub fn added_capabilities(&self, previous: &Self) -> Vec<Capability> {
        self.scopes
            .keys()
            .filter(|capability| !previous.scopes.contains_key(capability))
            .copied()
            .collect()
    }

    pub fn widened_capabilities(&self, previous: &Self) -> Vec<Capability> {
        self.scopes
            .iter()
            .filter(|(capability, scope)| {
                previous
                    .scopes
                    .get(capability)
                    .is_none_or(|old| !old.covers(scope))
            })
            .map(|(capability, _)| *capability)
            .collect()
    }

    fn declarations_value(&self) -> Value {
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
        Value::Object(root)
    }

    pub fn canonical_json(&self) -> String {
        let mut value = self.declarations_value();
        if let Some(object) = value.as_object_mut() {
            if let Some(artifact) = &self.artifact {
                object.insert("component".to_string(), Value::String(artifact.clone()));
            }
        }
        serde_json::to_string(&value).unwrap_or_default()
    }

    pub fn set_artifact_hash(&mut self, hash: &str) {
        self.artifact = Some(hash.to_string());
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
