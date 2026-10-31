use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::Value;

use kawi_plugin::{
    load_catalog, Capability, CapabilityRegistry, EffectiveCapabilities, HostEnvironment,
    InvocationContext, PluginHost, PluginHostEvent, PluginStatus, Position, Scope,
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub(super) struct TestHost {
    pub(super) host: PluginHost,
    pub(super) events: Receiver<PluginHostEvent>,
    pub(super) _wake: Receiver<()>,
    pub(super) root: PathBuf,
}

impl TestHost {
    pub(super) fn start(mode: &str, timeout_ms: u64) -> Self {
        Self::start_with(mode, timeout_ms, registry())
    }

    pub(super) fn start_with(
        mode: &str,
        timeout_ms: u64,
        capabilities: CapabilityRegistry,
    ) -> Self {
        let root = temp_dir();
        let executable = env!("CARGO_BIN_EXE_fake-plugin");
        write_manifest(&root, "fake", executable, mode, timeout_ms);
        Self::start_custom(root, capabilities)
    }

    pub(super) fn start_custom(root: PathBuf, capabilities: CapabilityRegistry) -> Self {
        let catalog = load_catalog(&root);
        assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
        let (event_tx, events) = mpsc::channel();
        let (wake, wake_rx) = mpsc::channel();
        let environment = HostEnvironment::from_paths(Some(root.clone()), None);
        let host = PluginHost::start(&catalog, capabilities, environment, event_tx, wake);
        Self {
            host,
            events,
            _wake: wake_rx,
            root,
        }
    }

    pub(super) fn invoke(&self) -> u64 {
        self.invoke_with(Value::Null)
    }

    pub(super) fn invoke_with(&self, args: Value) -> u64 {
        self.invoke_as("fake", args)
    }

    pub(super) fn invoke_as(&self, plugin: &str, args: Value) -> u64 {
        self.host
            .invoke(plugin, "probe", args, context())
            .expect("invoke")
    }

    pub(super) fn finish(&self, request: u64) -> Result<Value, kawi_plugin::PluginError> {
        self.finish_with_logs(request).0
    }

    pub(super) fn finish_with_logs(
        &self,
        request: u64,
    ) -> (Result<Value, kawi_plugin::PluginError>, Vec<String>) {
        let mut logs = Vec::new();
        loop {
            match self
                .events
                .recv_timeout(Duration::from_secs(3))
                .expect("event")
            {
                PluginHostEvent::InvocationFinished {
                    request: finished,
                    result,
                    ..
                } if finished == request => return (result, logs),
                PluginHostEvent::Log { line, .. } => logs.push(line),
                _ => continue,
            }
        }
    }

    pub(super) fn host_request(&self) -> (Value, String, Value) {
        loop {
            match self
                .events
                .recv_timeout(Duration::from_secs(3))
                .expect("event")
            {
                PluginHostEvent::HostRequest {
                    id, method, params, ..
                } => return (id, method, params),
                _ => continue,
            }
        }
    }
}

impl Drop for TestHost {
    fn drop(&mut self) {
        self.host.shutdown();
        fs::remove_dir_all(&self.root).ok();
    }
}

pub(super) fn temp_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("kawi-plugin-host-{}-{id}", std::process::id()));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub(super) fn write_manifest(
    root: &Path,
    name: &str,
    executable: &str,
    mode: &str,
    timeout_ms: u64,
) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let manifest = format!(
        "name = \"{name}\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [{executable:?}, {mode:?}]\ntimeout_ms = {timeout_ms}\nsubscribes = [\"buffer_changed\", \"file_saved\", \"mode_changed\", \"unknown_event\"]\n\n[[commands]]\nid = \"ping\"\ntitle = \"Ping\"\n"
    );
    fs::write(dir.join("plugin.toml"), manifest).unwrap();
}

pub(super) fn registry() -> CapabilityRegistry {
    registry_for(
        "fake",
        &[
            Capability::EditorRead,
            Capability::EditorWrite,
            Capability::UiMessage,
            Capability::DiagnosticsRead,
            Capability::DiagnosticsPublish,
        ],
    )
}

pub(super) fn registry_for(plugin: &str, capabilities: &[Capability]) -> CapabilityRegistry {
    let registry = CapabilityRegistry::default();
    let granted = capabilities
        .iter()
        .map(|capability| (*capability, Scope::Unscoped))
        .collect();
    registry.replace(std::collections::BTreeMap::from([(
        plugin.to_string(),
        EffectiveCapabilities { granted },
    )]));
    registry
}

pub(super) fn registry_without_editor_read() -> CapabilityRegistry {
    let registry = CapabilityRegistry::default();
    registry.replace(std::collections::BTreeMap::from([(
        "fake".to_string(),
        EffectiveCapabilities {
            granted: std::collections::BTreeMap::from([(Capability::UiMessage, Scope::Unscoped)]),
        },
    )]));
    registry
}

pub(super) fn context() -> InvocationContext {
    InvocationContext {
        buffer: 1,
        revision: 7,
        path: Some("/tmp/test.rs".to_string()),
        language: Some("rust".to_string()),
        mode: "normal".to_string(),
        cursor: Position {
            line: 0,
            byte_column: 0,
        },
        selection: None,
    }
}

pub(super) fn wait_for_retry(host: &TestHost) {
    if let Some(PluginStatus::Backoff { remaining_ms }) = host.host.status("fake") {
        std::thread::sleep(Duration::from_millis(remaining_ms.saturating_add(50)));
    }
}
