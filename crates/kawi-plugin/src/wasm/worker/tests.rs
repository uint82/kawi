use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::json;

use super::*;
use crate::capability::Capability;
use crate::manifest::{PluginManifest, PluginRuntime};
use crate::process::NotifyOutcome;

fn plugin() -> LoadedPlugin {
    plugin_with("echo.wasm")
}

fn plugin_with(component: &str) -> LoadedPlugin {
    plugin_with_subscriptions(component, Vec::new())
}

fn plugin_with_subscriptions(component: &str, subscribes: Vec<String>) -> LoadedPlugin {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "kawi-wasm-worker-{}-{id}-{component}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture(component), dir.join("plugin.wasm")).unwrap();
    LoadedPlugin {
        manifest: PluginManifest {
            name: "echo".to_string(),
            version: "0.1.0".to_string(),
            protocol: 1,
            description: String::new(),
            runtime: PluginRuntime::Wasm,
            component: Some("plugin.wasm".to_string()),
            sha256: None,
            command: Vec::new(),
            timeout_ms: 1_000,
            decorations: Vec::new(),
            settings: Default::default(),
            commands: Vec::new(),
            subscribes,
            debounce: Default::default(),
            activation_events: Vec::new(),
            capabilities: toml::Table::new(),
            capability_requests: Default::default(),
        },
        dir: dir.clone(),
        path: dir.join("plugin.toml"),
        computed_hash: None,
        disabled_reason: None,
    }
}

fn grants(plugin: &str, capabilities: &[Capability]) -> CapabilityRegistry {
    let registry = CapabilityRegistry::default();
    registry.replace(std::collections::BTreeMap::from([(
        plugin.to_string(),
        crate::capability::EffectiveCapabilities {
            granted: capabilities
                .iter()
                .map(|capability| (*capability, crate::capability::Scope::Unscoped))
                .collect(),
        },
    )]));
    registry
}

fn request(mut plugin: LoadedPlugin, capabilities: &[Capability]) -> LoadedPlugin {
    let mut table = toml::Table::new();
    for capability in capabilities {
        table.insert(capability.name().to_string(), toml::Value::Boolean(true));
    }
    plugin.manifest.capability_requests = crate::capability::parse_capability_requests(&table).0;
    plugin
}

fn start(
    plugin: LoadedPlugin,
    capabilities: CapabilityRegistry,
) -> (WasmWorkerHandle, mpsc::Receiver<PluginHostEvent>) {
    let (events_tx, events_rx) = mpsc::channel();
    let (wake_tx, _wake_rx) = mpsc::channel();
    let handle = start_wasm_worker(
        plugin,
        capabilities,
        HostEnvironment::default(),
        events_tx,
        wake_tx,
    );
    (handle, events_rx)
}

fn request_paths(mut plugin: LoadedPlugin, paths: &[(Capability, &str)]) -> LoadedPlugin {
    let mut requests = toml::Table::new();
    for (capability, path) in paths {
        let mut scope = toml::Table::new();
        scope.insert(
            "paths".to_string(),
            toml::Value::Array(vec![toml::Value::String((*path).to_string())]),
        );
        requests.insert(capability.name().to_string(), toml::Value::Table(scope));
    }
    plugin.manifest.capability_requests = crate::capability::parse_capability_requests(&requests).0;
    plugin
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/wasm")
        .join(name)
}

#[test]
fn invokes_a_wasm_component() {
    let plugin = plugin();
    let dir = plugin.dir.clone();
    let (events_tx, events_rx) = mpsc::channel();
    let (wake_tx, _wake_rx) = mpsc::channel();
    let handle = start_wasm_worker(
        plugin,
        CapabilityRegistry::default(),
        HostEnvironment::default(),
        events_tx,
        wake_tx,
    );
    handle
        .invoke(
            7,
            "ping".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let event = events_rx
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        match event {
            PluginHostEvent::InvocationFinished { result, .. } => {
                assert_eq!(result.unwrap(), json!("pong"));
                break;
            }
            PluginHostEvent::Log { line, .. } => panic!("plugin log: {line}"),
            _ => {}
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn stops_a_component_that_exceeds_its_wall_clock() {
    let plugin = plugin_with("loop.wasm");
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let (events_tx, events_rx) = mpsc::channel();
    let (wake_tx, _wake_rx) = mpsc::channel();
    let handle = start_wasm_worker(
        plugin,
        grants(&name, &[]),
        HostEnvironment::default(),
        events_tx,
        wake_tx,
    );
    handle
        .invoke(
            11,
            "spin".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let event = events_rx
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        if let PluginHostEvent::InvocationFinished { result, .. } = event {
            let error = result.unwrap_err();
            assert_eq!(error, PluginError::Timeout);
            break;
        }
    }
    assert!(matches!(
        handle.status(),
        PluginStatus::Backoff { .. } | PluginStatus::Disabled
    ));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn forwards_events_to_a_wasm_component() {
    let plugin = request(
        plugin_with_subscriptions("events.wasm", vec!["file_saved".to_string()]),
        &[Capability::UiMessage],
    );
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let capabilities = grants(&name, &[Capability::EditorRead, Capability::UiMessage]);
    let (events_tx, events_rx) = mpsc::channel();
    let (wake_tx, _wake_rx) = mpsc::channel();
    let handle = start_wasm_worker(
        plugin,
        capabilities,
        HostEnvironment::default(),
        events_tx,
        wake_tx,
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    handle
        .invoke(
            9,
            "ping".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    loop {
        let event = events_rx
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        match event {
            PluginHostEvent::InvocationFinished { result, .. } => {
                assert_eq!(result.unwrap(), json!("pong"));
                break;
            }
            PluginHostEvent::Log { line, .. } => panic!("plugin log: {line}"),
            _ => {}
        }
    }
    assert_eq!(
        handle.notify(
            "file_saved",
            json!({"buffer": 1, "path": null, "revision": 2})
        ),
        NotifyOutcome::Queued
    );
    loop {
        let event = events_rx
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        if let PluginHostEvent::HostRequest { id, method, .. } = event {
            assert_eq!(method, "ui.message");
            handle.respond(id, Ok(json!(null))).unwrap();
            break;
        }
    }
    handle
        .invoke(
            10,
            "ping".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    loop {
        let event = events_rx
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        if let PluginHostEvent::InvocationFinished { result, .. } = event {
            assert_eq!(result.unwrap(), json!("pong"));
            break;
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

mod editor;
mod events;
#[cfg(unix)]
mod fs;
mod imports;
mod timeout;
