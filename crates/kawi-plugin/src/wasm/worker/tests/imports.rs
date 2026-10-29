use std::collections::BTreeSet;
use std::time::Duration;

use serde_json::json;

use super::*;
use crate::capability::Capability;
use crate::error::PluginError;

#[test]
fn refuses_an_unrequested_import() {
    let plugin = plugin_with("editor-host.wasm");
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
            12,
            "read".to_string(),
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
            let error = result.unwrap_err().to_string();
            assert!(error.contains("not requested"), "{error}");
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
fn requests_a_missing_import_grant() {
    let plugin = request(plugin_with("editor-host.wasm"), &[Capability::EditorRead]);
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
            13,
            "read".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut requested = false;
    loop {
        let event = events_rx
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        match event {
            PluginHostEvent::CapabilityRequired { plugin, capability } => {
                assert_eq!(plugin, name);
                assert_eq!(capability, Capability::EditorRead);
                requested = true;
            }
            PluginHostEvent::InvocationFinished { result, .. } => {
                let error = result.unwrap_err();
                assert_eq!(
                    error,
                    PluginError::MissingCapabilities(vec![Capability::EditorRead])
                );
                assert_eq!(
                    error.to_string(),
                    "plugin requests editor.read, not granted"
                );
                break;
            }
            _ => {}
        }
    }
    assert!(requested);
    assert!(matches!(
        handle.status(),
        PluginStatus::Backoff { .. } | PluginStatus::Disabled
    ));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn rejects_an_import_outside_the_manifest_without_prompting() {
    let plugin = request(plugin_with("fs-sandbox.wasm"), &[Capability::EditorRead]);
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let (handle, events_rx) = start(plugin, grants(&name, &[]));
    handle
        .invoke(
            14,
            "run".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let (requested, error) = wait_for_import_failure(&events_rx);
    assert!(requested.is_empty());
    assert!(error.to_string().contains("not requested"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn requests_only_the_manifest_filesystem_alternative() {
    let path = std::env::temp_dir();
    let plugin = request_paths(
        plugin_with("fs-sandbox.wasm"),
        Capability::FsRead,
        &path.to_string_lossy(),
    );
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let (handle, events_rx) = start(plugin, grants(&name, &[]));
    handle
        .invoke(
            15,
            "run".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let (requested, error) = wait_for_import_failure(&events_rx);
    assert_eq!(requested, BTreeSet::from([Capability::FsRead]), "{error}");
    assert_eq!(
        error,
        PluginError::MissingCapabilities(vec![Capability::FsRead])
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn reports_each_missing_capability_once() {
    let plugin = request(
        plugin_with("multi-import.wasm"),
        &[
            Capability::EditorRead,
            Capability::UiMessage,
            Capability::DiagnosticsPublish,
        ],
    );
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let (handle, events_rx) = start(plugin, grants(&name, &[]));
    handle
        .invoke(
            16,
            "ping".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let (requested, error) = wait_for_import_failure(&events_rx);
    assert_eq!(
        requested,
        BTreeSet::from([
            Capability::EditorRead,
            Capability::UiMessage,
            Capability::DiagnosticsPublish,
        ])
    );
    assert_eq!(
        error,
        PluginError::MissingCapabilities(vec![
            Capability::EditorRead,
            Capability::UiMessage,
            Capability::DiagnosticsPublish,
        ])
    );
    let _ = std::fs::remove_dir_all(dir);
}

fn wait_for_import_failure(
    events: &std::sync::mpsc::Receiver<PluginHostEvent>,
) -> (BTreeSet<Capability>, PluginError) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut requested = BTreeSet::new();
    loop {
        let event = events
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        match event {
            PluginHostEvent::CapabilityRequired { capability, .. } => {
                requested.insert(capability);
            }
            PluginHostEvent::InvocationFinished { result, .. } => {
                return (requested, result.unwrap_err());
            }
            _ => {}
        }
    }
}
