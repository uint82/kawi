use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use kawi_plugin::PluginStatus;

use super::common::*;

#[test]
fn inspect_reports_a_wasm_manifest_and_hash() {
    let root = temp_dir("inspect-wasm");
    let hash = write_plugin(&root, None);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(app.plugins.status("wasm"), Some(PluginStatus::Disabled));
    app.handle_plugin_command("inspect wasm");
    let lines = app.pager.as_ref().expect("pager");
    assert!(lines.iter().any(|line| line == "runtime: wasm"));
    assert!(lines.iter().any(|line| line == "component: plugin.wasm"));
    assert!(lines
        .iter()
        .any(|line| line.contains(&format!("sha256: sha256:{hash}"))));
    assert!(lines
        .iter()
        .any(|line| line.contains("requests: editor.read")));
    #[cfg(feature = "wasm")]
    assert!(lines.iter().any(|line| line == "imports:"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inspect_reports_the_computed_hash_when_the_manifest_lies() {
    let root = temp_dir("inspect-wasm-mismatch");
    let hash = write_plugin(&root, Some(&"0".repeat(64)));
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("hash mismatch")));
    assert_eq!(app.plugins.status("wasm"), Some(PluginStatus::Disabled));
    app.handle_plugin_command("inspect wasm");
    let lines = app.pager.as_ref().expect("pager");
    assert!(lines
        .iter()
        .any(|line| line.contains(&format!("sha256: sha256:{hash}"))));
    assert!(lines
        .iter()
        .any(|line| line.contains(&format!("declared sha256: sha256:{}", "0".repeat(64)))));
    assert!(lines.iter().any(|line| line.contains("disabled:")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_wasm_is_reported_as_disabled() {
    let root = temp_dir("inspect-wasm-malformed");
    write_plugin_bytes(&root, b"not a component", None);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("not a WebAssembly component")));
    assert_eq!(app.plugins.status("wasm"), Some(PluginStatus::Disabled));
    app.handle_plugin_command("inspect wasm");
    let lines = app.pager.as_ref().expect("pager");
    assert!(lines.iter().any(|line| line.contains("disabled:")));
    fs::remove_dir_all(root).unwrap();
}

fn write_plugin(root: &Path, declared: Option<&str>) -> String {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kawi-plugin/tests/fixtures/wasm/editor-read.wasm");
    let bytes = fs::read(fixture).unwrap();
    write_plugin_bytes(root, &bytes, declared)
}

fn write_plugin_bytes(root: &Path, bytes: &[u8], declared: Option<&str>) -> String {
    let dir = root.join("wasm");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("plugin.wasm"), bytes).unwrap();
    let hash = format!("{:x}", Sha256::digest(bytes));
    let declared = declared.unwrap_or(&hash);
    fs::write(
        dir.join("plugin.toml"),
        format!(
            "name = \"wasm\"\nversion = \"0.1.0\"\nprotocol = 1\nruntime = \"wasm\"\ncomponent = \"plugin.wasm\"\nsha256 = \"{declared}\"\n\n[capabilities]\n\"editor.read\" = true\n"
        ),
    )
    .unwrap();
    hash
}

#[cfg(feature = "wasm")]
#[test]
fn inspect_detects_a_component_replaced_after_load() {
    let root = temp_dir("inspect-wasm-replaced");
    write_plugin(&root, None);
    let mut app = app();
    load_plugins(&mut app, &root);
    fs::write(root.join("wasm/plugin.wasm"), b"replaced").unwrap();
    app.handle_plugin_command("inspect wasm");
    let lines = app.pager.as_ref().expect("pager");
    assert!(lines.iter().any(|line| line.contains("changed since load")));
    fs::remove_dir_all(root).unwrap();
}
