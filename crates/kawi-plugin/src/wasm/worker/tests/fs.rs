use std::collections::BTreeMap;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use serde_json::json;

use super::*;
use crate::capability::{EffectiveCapabilities, Scope};

#[cfg(unix)]
#[test]
fn blocks_preopen_escapes_and_read_only_writes() {
    use std::os::unix::fs::symlink;

    let (root, allowed) = sandbox_root();
    fs::write(root.join("outside.txt"), "outside").unwrap();
    symlink(root.join("outside.txt"), allowed.join("link")).unwrap();

    let path = allowed.to_string_lossy();
    let plugin = fs_plugin("fs-sandbox.wasm", &path);
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let result = run_probe(
        plugin,
        read_capabilities(&name, &path),
        HostEnvironment::default(),
    );
    assert_eq!(result, json!("blocked"));
    assert!(!allowed.join("write-test.txt").exists());

    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn expands_workspace_in_preopens() {
    let (root, allowed) = sandbox_root();
    fs::write(root.join("outside.txt"), "outside").unwrap();

    let plugin = fs_plugin("fs-sandbox.wasm", "$WORKSPACE");
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let environment = HostEnvironment {
        workspace: Some(allowed.clone()),
        ..Default::default()
    };
    let result = run_probe(plugin, read_capabilities(&name, "$WORKSPACE"), environment);
    assert_eq!(result, json!("blocked"));
    assert!(!allowed.join("write-test.txt").exists());

    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn skips_missing_preopens_with_a_log() {
    let (root, allowed) = sandbox_root();
    let plugin = fs_plugin("fs-sandbox.wasm", "$WORKSPACE/missing");
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let environment = HostEnvironment {
        workspace: Some(allowed.clone()),
        ..Default::default()
    };
    let probe = run_probe_events(
        plugin,
        read_capabilities(&name, "$WORKSPACE/missing"),
        environment,
    );
    assert_eq!(probe.result, json!("no-preopen"));
    assert!(probe
        .logs
        .iter()
        .any(|line| line.contains("skipped granted fs.read path")));

    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn skips_file_preopens_with_a_log() {
    let (root, allowed) = sandbox_root();
    fs::write(allowed.join("data.txt"), "data").unwrap();
    let plugin = fs_plugin("fs-sandbox.wasm", "$WORKSPACE/data.txt");
    let dir = plugin.dir.clone();
    let name = plugin.manifest.name.clone();
    let environment = HostEnvironment {
        workspace: Some(allowed.clone()),
        ..Default::default()
    };
    let probe = run_probe_events(
        plugin,
        read_capabilities(&name, "$WORKSPACE/data.txt"),
        environment,
    );
    assert_eq!(probe.result, json!("no-preopen"));
    assert!(probe
        .logs
        .iter()
        .any(|line| line.contains("WASI preopens require a directory")));

    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
fn sandbox_root() -> (std::path::PathBuf, std::path::PathBuf) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "kawi-wasm-fs-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let allowed = root.join("allowed");
    fs::create_dir_all(&allowed).unwrap();
    (root, allowed)
}

struct Probe {
    result: serde_json::Value,
    logs: Vec<String>,
}

fn run_probe(
    plugin: LoadedPlugin,
    capabilities: CapabilityRegistry,
    environment: HostEnvironment,
) -> serde_json::Value {
    run_probe_events(plugin, capabilities, environment).result
}

fn run_probe_events(
    plugin: LoadedPlugin,
    capabilities: CapabilityRegistry,
    environment: HostEnvironment,
) -> Probe {
    let (events_tx, events_rx) = std::sync::mpsc::channel();
    let (wake_tx, _wake_rx) = std::sync::mpsc::channel();
    let handle = start_wasm_worker(plugin, capabilities, environment, events_tx, wake_tx);
    handle
        .invoke(
            1,
            "probe".to_string(),
            json!([]),
            InvocationContext::redacted(),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut logs = Vec::new();
    loop {
        let event = events_rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        match event {
            PluginHostEvent::Log { line, .. } => logs.push(line),
            PluginHostEvent::InvocationFinished { result, .. } => {
                return Probe {
                    result: result.unwrap(),
                    logs,
                };
            }
            _ => {}
        }
    }
}

fn read_capabilities(plugin: &str, path: &str) -> CapabilityRegistry {
    let registry = CapabilityRegistry::default();
    registry.replace(BTreeMap::from([(
        plugin.to_string(),
        EffectiveCapabilities {
            granted: BTreeMap::from([(Capability::FsRead, Scope::Paths(vec![path.to_string()]))]),
            ..Default::default()
        },
    )]));
    registry
}

fn fs_plugin(component: &str, path: &str) -> LoadedPlugin {
    request_paths(
        plugin_with(component),
        &[(Capability::FsRead, path), (Capability::FsWrite, path)],
    )
}
