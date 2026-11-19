use std::time::{Duration, Instant};

use serde_json::json;

use super::*;
use crate::protocol::INVALID_PARAMS;

const TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn guest_errors_keep_the_session_alive() {
    let plugin = request(
        plugin_with("autocmds.wasm"),
        &[Capability::EditorAutocmds, Capability::UiMessage],
    );
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let capabilities = grants(&name, &[Capability::EditorAutocmds, Capability::UiMessage]);
    let (handle, events_rx) = start(plugin, capabilities);

    for request in [1, 2] {
        let error = invoke_error(&handle, &events_rx, request, "missing");
        assert!(
            matches!(&error, PluginError::Rpc(rpc) if rpc.code == INVALID_PARAMS),
            "{error}"
        );
        assert_eq!(handle.status(), PluginStatus::Ready);
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn malformed_events_keep_the_session_alive() {
    let plugin = plugin_with_subscriptions("autocmds.wasm", vec!["file_saved".to_string()]);
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let capabilities = grants(&name, &[Capability::EditorAutocmds, Capability::UiMessage]);
    let (handle, events_rx) = start(
        request(plugin, &[Capability::EditorAutocmds, Capability::UiMessage]),
        capabilities,
    );

    assert_eq!(
        handle.notify("file_saved", json!({})),
        NotifyOutcome::Queued
    );
    assert_eq!(
        handle.notify("file_saved", json!({})),
        NotifyOutcome::Queued
    );
    let error = invoke_error(&handle, &events_rx, 3, "missing");
    assert!(
        matches!(&error, PluginError::Rpc(rpc) if rpc.code == INVALID_PARAMS),
        "{error}"
    );
    assert_eq!(handle.status(), PluginStatus::Ready);
    let _ = std::fs::remove_dir_all(dir);
}

fn invoke_error(
    handle: &WasmWorkerHandle,
    events: &mpsc::Receiver<PluginHostEvent>,
    request: u64,
    command: &str,
) -> PluginError {
    handle
        .invoke(
            request,
            command.to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let event = events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if let PluginHostEvent::InvocationFinished { result, .. } = event {
            return result.unwrap_err();
        }
    }
}
