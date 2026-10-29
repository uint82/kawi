use std::time::{Duration, Instant};

use serde_json::json;

use super::*;
use crate::wasm::WasmRuntime;

#[test]
fn starts_fresh_after_a_blocked_host_call_times_out() {
    let plugin = request(plugin_with("editor-host.wasm"), &[Capability::EditorRead]);
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let capabilities = grants(&name, &[Capability::EditorRead]);
    let (handle, events_rx) = start(plugin, capabilities);
    handle
        .invoke(
            1,
            "read".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    wait_for_host_request(&events_rx, "editor.get_state");
    let started = Instant::now();
    let error = wait_for_invocation_error(&events_rx);
    assert_eq!(error, PluginError::Timeout);
    assert!(started.elapsed() < Duration::from_secs(4));
    assert!(matches!(
        handle.status(),
        PluginStatus::Backoff { .. } | PluginStatus::Disabled
    ));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn restart_cancels_a_blocked_host_call() {
    let plugin = request(plugin_with("editor-host.wasm"), &[Capability::EditorRead]);
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let capabilities = grants(&name, &[Capability::EditorRead]);
    let (handle, events_rx) = start(plugin, capabilities);
    handle
        .invoke(
            2,
            "read".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    wait_for_host_request(&events_rx, "editor.get_state");
    let started = Instant::now();
    handle.restart().unwrap();
    let error = wait_for_invocation_error(&events_rx);
    assert!(error.to_string().contains("restart"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(wait_for_status(&handle, PluginStatus::Ready));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn reports_a_requested_import_without_a_grant() {
    let plugin = request(plugin_with("editor-host.wasm"), &[Capability::EditorRead]);
    let dir = plugin.dir.clone();
    let (handle, events_rx) = start(plugin, CapabilityRegistry::default());
    handle
        .invoke(
            3,
            "read".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let error = wait_for_invocation_error(&events_rx);
    assert!(
        error
            .to_string()
            .contains("plugin requests editor.read, not granted"),
        "{error}"
    );
    assert!(matches!(
        handle.status(),
        PluginStatus::Backoff { .. } | PluginStatus::Disabled
    ));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn rejects_component_bytes_that_changed_after_load() {
    let mut plugin = plugin_with("echo.wasm");
    let dir = plugin.dir.clone();
    let hash = WasmRuntime::new()
        .unwrap()
        .inspect(&fixture("echo.wasm"), None, &[])
        .unwrap()
        .hash;
    plugin.computed_hash = Some(hash);
    std::fs::write(dir.join("plugin.wasm"), b"changed").unwrap();
    let (handle, events_rx) = start(plugin, CapabilityRegistry::default());
    handle
        .invoke(
            4,
            "ping".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let error = wait_for_invocation_error(&events_rx);
    assert!(error.to_string().contains("hash changed"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}

fn wait_for_host_request(
    events: &std::sync::mpsc::Receiver<PluginHostEvent>,
    method: &str,
) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let event = events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if let PluginHostEvent::HostRequest {
            id,
            method: requested,
            ..
        } = event
        {
            if requested == method {
                return id;
            }
        }
    }
}

fn wait_for_invocation_error(events: &std::sync::mpsc::Receiver<PluginHostEvent>) -> PluginError {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let event = events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if let PluginHostEvent::InvocationFinished { result, .. } = event {
            return result.unwrap_err();
        }
    }
}

fn wait_for_status(handle: &WasmWorkerHandle, expected: PluginStatus) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if handle.status() == expected {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn discards_the_store_after_a_wall_clock_trap() {
    let mut plugin = plugin_with("loop.wasm");
    let dir = plugin.dir.clone();
    let hash = WasmRuntime::new()
        .unwrap()
        .inspect(&fixture("loop.wasm"), None, &[])
        .unwrap()
        .hash;
    plugin.computed_hash = Some(hash);
    let (handle, events_rx) = start(plugin, CapabilityRegistry::default());
    handle
        .invoke(
            5,
            "loop".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let error = wait_for_invocation_error(&events_rx);
    assert_eq!(error, PluginError::Timeout);
    std::fs::write(dir.join("plugin.wasm"), b"changed").unwrap();
    std::thread::sleep(Duration::from_millis(400));
    handle
        .invoke(
            6,
            "loop".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let error = wait_for_invocation_error(&events_rx);
    assert!(error.to_string().contains("hash changed"), "{error}");
    let _ = std::fs::remove_dir_all(dir);
}
