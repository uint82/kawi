use std::fs;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::json;
use sha2::{Digest, Sha256};

use kawi_plugin::{
    load_catalog, CapabilityRegistry, HostEnvironment, InvocationContext, PluginHost,
    PluginHostEvent, PluginStatus,
};

use super::common::temp_dir;

#[test]
fn invokes_a_wasm_plugin_through_the_host() {
    let root = temp_dir();
    let dir = root.join("echo");
    fs::create_dir_all(&dir).unwrap();
    let bytes = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/wasm/echo.wasm"
    ))
    .unwrap();
    fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
    let digest = Sha256::digest(&bytes);
    fs::write(
        dir.join("plugin.toml"),
        format!(
            "name = \"echo\"\nversion = \"0.1.0\"\nprotocol = 1\nruntime = \"wasm\"\ncomponent = \"plugin.wasm\"\nsha256 = \"sha256:{digest:x}\"\n"
        ),
    )
    .unwrap();
    let catalog = load_catalog(&root);
    assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
    let (event_tx, events) = mpsc::channel();
    let (wake, _wake_rx) = mpsc::channel();
    let host = PluginHost::start(
        &catalog,
        CapabilityRegistry::default(),
        HostEnvironment::default(),
        event_tx,
        wake,
    );
    let request = host
        .invoke("echo", "ping", json!([]), InvocationContext::redacted())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let event = events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        match event {
            PluginHostEvent::InvocationFinished { result, .. } => {
                assert_eq!(result.unwrap(), json!("pong"));
                break;
            }
            PluginHostEvent::Log { line, .. } => panic!("plugin log: {line}"),
            PluginHostEvent::StatusChanged { status, .. } => {
                assert_ne!(status, PluginStatus::Error("unexpected".to_string()));
            }
            _ => {}
        }
    }
    assert_eq!(host.status("echo"), Some(PluginStatus::Ready));
    assert_eq!(request, 1);
    host.shutdown();
    fs::remove_dir_all(root).unwrap();
}
