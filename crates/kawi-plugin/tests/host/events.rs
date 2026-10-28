use std::fs;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::json;

use kawi_plugin::{load_catalog, NotifyOutcome, PluginHost, PluginStatus};

use super::common::*;

#[test]
fn unknown_event_subscriptions_are_ignored() {
    let host = TestHost::start("normal", 1_000);
    assert_eq!(
        host.host
            .notify("fake", "buffer_changed", json!({"buffer": 1}))
            .unwrap(),
        NotifyOutcome::NotRunning
    );
    host.host.activate("fake").unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while host.host.status("fake") != Some(PluginStatus::Ready)
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        host.host
            .notify("fake", "unknown_event", json!({"buffer": 1}))
            .unwrap(),
        NotifyOutcome::NotSubscribed
    );
    assert_eq!(
        host.host
            .notify("fake", "buffer_changed", json!({"buffer": 1}))
            .unwrap(),
        NotifyOutcome::Queued
    );
}

#[test]
fn events_flow_while_an_invocation_is_running() {
    let host = TestHost::start("host-call", 1_000);
    let request = host.invoke();
    let (id, _, _) = host.host_request();
    assert_eq!(
        host.host
            .notify(
                "fake",
                "buffer_changed",
                json!({"buffer": 1, "revision": 2})
            )
            .unwrap(),
        NotifyOutcome::Queued
    );
    host.host
        .respond("fake", id, Ok(json!({"revision": 1})))
        .unwrap();
    assert!(host.finish(request).is_ok());
}

#[test]
fn event_flood_against_a_stalled_plugin_drops_without_blocking() {
    let host = TestHost::start("stalled", 1_000);
    host.host.activate("fake").unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while host.host.status("fake") != Some(PluginStatus::Ready)
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(host.host.status("fake"), Some(PluginStatus::Ready));
    let started = std::time::Instant::now();
    let mut overflow = 0;
    for revision in 0..10_000 {
        if host
            .host
            .notify("fake", "buffer_changed", json!({"revision": revision}))
            .unwrap()
            == NotifyOutcome::Overflow
        {
            overflow += 1;
        }
    }
    assert!(overflow > 0);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn activation_starts_a_plugin_without_a_command() {
    let root = temp_dir();
    let executable = env!("CARGO_BIN_EXE_fake-plugin");
    let dir = root.join("fake");
    fs::create_dir_all(&dir).unwrap();
    let manifest = format!(
        "name = \"fake\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [{executable:?}, \"normal\" ]\ntimeout_ms = 1000\n\n[[activation_events]]\nevent = \"on_startup\"\n\n[[commands]]\nid = \"ping\"\ntitle = \"Ping\"\n"
    );
    fs::write(dir.join("plugin.toml"), manifest).unwrap();
    let catalog = load_catalog(&root);
    assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
    let (event_tx, _events) = mpsc::channel();
    let (wake, _wake_rx) = mpsc::channel();
    let host = PluginHost::start(&catalog, registry(), event_tx, wake);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while host.status("fake") != Some(PluginStatus::Ready) && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(host.status("fake"), Some(PluginStatus::Ready));
    host.shutdown();
    fs::remove_dir_all(root).ok();
}
