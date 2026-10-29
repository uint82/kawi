use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::*;
use crate::capability::parse_capability_requests;

fn wasm_manifest(name: &str, hash: &str) -> String {
    format!(
        r#"name = "{name}"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "plugin.wasm"
sha256 = "{hash}"

[capabilities]
"editor.read" = true

[[commands]]
id = "ping"
title = "Ping"
"#
    )
}

fn write_component(dir: &Path, bytes: &[u8]) -> String {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("plugin.wasm"), bytes).unwrap();
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

#[test]
fn loads_a_wasm_manifest_and_hashes_the_component() {
    let root = temp_dir();
    let dir = root.join("wasm");
    let hash = write_component(
        &dir,
        include_bytes!("../../../tests/fixtures/wasm/editor-read.wasm"),
    );
    fs::write(dir.join(MANIFEST_FILE), wasm_manifest("wasm", &hash)).unwrap();
    let catalog = load_catalog(&root);
    assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
    let plugin = &catalog.plugins[0];
    assert_eq!(plugin.manifest.runtime, PluginRuntime::Wasm);
    assert_eq!(
        plugin.component_hash(),
        Some(format!("sha256:{hash}").as_str())
    );
    assert!(plugin.component_path().is_some());
    assert_ne!(
        plugin.manifest.capability_requests.request_hash(),
        process_request_hash()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_component_hash_mismatch() {
    let root = temp_dir();
    let dir = root.join("wasm");
    write_component(&dir, b"component");
    fs::write(
        dir.join(MANIFEST_FILE),
        wasm_manifest("wasm", &"0".repeat(64)),
    )
    .unwrap();
    let catalog = load_catalog(&root);
    assert_eq!(catalog.plugins.len(), 1);
    assert!(catalog.plugins[0]
        .disabled_reason
        .as_deref()
        .unwrap()
        .contains("hash mismatch"));
    assert!(catalog.plugins[0].component_hash().is_some());
    assert!(catalog.warnings[0].message.contains("hash mismatch"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_component_paths_outside_the_plugin_directory() {
    let root = temp_dir();
    let dir = root.join("wasm");
    let hash = write_component(&dir, b"component");
    let body = wasm_manifest("wasm", &hash).replace(
        "component = \"plugin.wasm\"",
        "component = \"../plugin.wasm\"",
    );
    fs::write(dir.join(MANIFEST_FILE), body).unwrap();
    let catalog = load_catalog(&root);
    assert_eq!(catalog.plugins.len(), 1);
    assert!(catalog.plugins[0]
        .disabled_reason
        .as_deref()
        .unwrap()
        .contains("inside the plugin directory"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn process_manifests_reject_component_fields() {
    let root = temp_dir();
    let body =
        valid_manifest("echo").replace("protocol = 1", "protocol = 1\ncomponent = \"plugin.wasm\"");
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert!(catalog.plugins.is_empty());
    assert!(catalog.warnings[0]
        .message
        .contains("process plugins cannot"));
    fs::remove_dir_all(root).unwrap();
}

fn process_request_hash() -> String {
    let table: toml::Table = toml::from_str("\"editor.read\" = true").unwrap();
    let (requests, warnings) = parse_capability_requests(&table);
    assert!(warnings.is_empty());
    requests.request_hash()
}

#[cfg(unix)]
#[test]
fn rejects_component_symlinks_outside_the_plugin_directory() {
    use std::os::unix::fs::symlink;

    let root = temp_dir();
    let dir = root.join("wasm");
    fs::create_dir_all(&dir).unwrap();
    let outside = root.join("outside.wasm");
    fs::write(&outside, b"component").unwrap();
    symlink(&outside, dir.join("plugin.wasm")).unwrap();
    let hash = {
        let digest = Sha256::digest(b"component");
        format!("{digest:x}")
    };
    fs::write(dir.join(MANIFEST_FILE), wasm_manifest("wasm", &hash)).unwrap();
    let catalog = load_catalog(&root);
    assert_eq!(catalog.plugins.len(), 1);
    assert!(catalog.plugins[0]
        .disabled_reason
        .as_deref()
        .unwrap()
        .contains("inside the plugin directory"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn disables_malformed_components() {
    let root = temp_dir();
    let dir = root.join("wasm");
    let hash = write_component(&dir, b"not a component");
    fs::write(dir.join(MANIFEST_FILE), wasm_manifest("wasm", &hash)).unwrap();
    let catalog = load_catalog(&root);
    assert_eq!(catalog.plugins.len(), 1);
    assert!(catalog.plugins[0]
        .disabled_reason
        .as_deref()
        .unwrap()
        .contains("not a WebAssembly component"));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(feature = "wasm")]
#[test]
fn disables_truncated_components_after_the_header() {
    let root = temp_dir();
    let dir = root.join("wasm");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/wasm/editor-read.wasm");
    let bytes = fs::read(fixture).unwrap();
    let hash = write_component(&dir, &bytes[..bytes.len() - 4]);
    fs::write(dir.join(MANIFEST_FILE), wasm_manifest("wasm", &hash)).unwrap();
    let catalog = load_catalog(&root);
    assert_eq!(catalog.plugins.len(), 1);
    let reason = catalog.plugins[0].disabled_reason.as_deref().unwrap();
    assert!(reason.contains("invalid component"), "{reason}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_absolute_component_paths() {
    let root = temp_dir();
    let dir = root.join("wasm");
    let hash = write_component(&dir, b"component");
    let body = wasm_manifest("wasm", &hash).replace(
        "component = \"plugin.wasm\"",
        "component = \"/tmp/plugin.wasm\"",
    );
    fs::write(dir.join(MANIFEST_FILE), body).unwrap();
    let catalog = load_catalog(&root);
    assert_eq!(catalog.plugins.len(), 1);
    assert!(catalog.plugins[0]
        .disabled_reason
        .as_deref()
        .unwrap()
        .contains("relative to the plugin directory"));
    fs::remove_dir_all(root).unwrap();
}
