use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use serde_json::json;

use super::*;
use crate::capability::{EffectiveCapabilities, Scope};

#[cfg(unix)]
#[test]
fn blocks_preopen_escapes_and_read_only_writes() {
    use std::os::unix::fs::symlink;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "kawi-wasm-fs-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let allowed = root.join("allowed");
    fs::create_dir_all(&allowed).unwrap();
    fs::write(root.join("outside.txt"), "outside").unwrap();
    symlink(root.join("outside.txt"), allowed.join("link")).unwrap();

    let plugin = fs_plugin("fs-sandbox.wasm", &allowed);
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let capabilities = CapabilityRegistry::default();
    capabilities.replace(BTreeMap::from([(
        name.clone(),
        EffectiveCapabilities {
            granted: BTreeMap::from([(
                Capability::FsRead,
                Scope::Paths(vec![allowed.to_string_lossy().into_owned()]),
            )]),
            ..Default::default()
        },
    )]));
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
            "probe".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let result = loop {
        let event = events_rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if let PluginHostEvent::InvocationFinished { result, .. } = event {
            break result.unwrap();
        }
    };
    assert_eq!(result, json!("blocked"));
    assert!(!allowed.join("write-test.txt").exists());

    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(root);
}

fn fs_plugin(component: &str, allowed: &Path) -> LoadedPlugin {
    request_paths(
        plugin_with(component),
        Capability::FsRead,
        &allowed.to_string_lossy(),
    )
}
