use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde_json::json;

use super::*;

#[test]
fn revoking_editor_read_drops_queued_events() {
    let plugin = request(
        plugin_with_subscriptions("events.wasm", vec!["buffer_changed".to_string()]),
        &[Capability::UiMessage],
    );
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let capabilities = grants(&name, &[Capability::EditorRead, Capability::UiMessage]);
    let registry = capabilities.clone();
    let (events_tx, events_rx) = std::sync::mpsc::channel();
    let (wake_tx, _wake_rx) = std::sync::mpsc::channel();
    let handle = start_wasm_worker(
        plugin,
        capabilities,
        HostEnvironment::default(),
        events_tx,
        wake_tx,
    );
    handle
        .invoke(
            1,
            "ping".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    wait_for_invocation(&events_rx);
    assert_eq!(
        handle.notify(
            "buffer_changed",
            json!({
                "buffer": 1,
                "revision": 2,
                "previous_revision": 1,
                "origin": "test",
                "full_resync": false,
                "changes": []
            }),
        ),
        NotifyOutcome::Queued
    );
    let id = wait_for_message(&events_rx);
    assert_eq!(
        handle.notify(
            "buffer_changed",
            json!({
                "buffer": 1,
                "revision": 3,
                "previous_revision": 2,
                "origin": "test",
                "full_resync": false,
                "changes": []
            }),
        ),
        NotifyOutcome::Queued
    );
    registry.replace(BTreeMap::new());
    handle.respond(id, Ok(json!(null))).unwrap();
    assert!(no_message(&events_rx, Duration::from_millis(500)));
    let _ = std::fs::remove_dir_all(dir);
}

fn wait_for_invocation(events: &std::sync::mpsc::Receiver<PluginHostEvent>) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let event = events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if let PluginHostEvent::InvocationFinished { result, .. } = event {
            assert_eq!(result.unwrap(), json!("pong"));
            return;
        }
    }
}

fn wait_for_message(events: &std::sync::mpsc::Receiver<PluginHostEvent>) -> Value {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let event = events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if let PluginHostEvent::HostRequest { id, method, .. } = event {
            if method == "ui.message" {
                return id;
            }
        }
    }
}

fn no_message(events: &std::sync::mpsc::Receiver<PluginHostEvent>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return true;
        }
        match events.recv_timeout(remaining) {
            Ok(PluginHostEvent::HostRequest { method, .. }) if method == "ui.message" => {
                return false;
            }
            Ok(_) => {}
            Err(_) => return true,
        }
    }
}
