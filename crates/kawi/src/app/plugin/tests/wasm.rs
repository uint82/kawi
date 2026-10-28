use std::fs;

use sha2::{Digest, Sha256};

use kawi_plugin::PluginStatus;

use super::common::*;

#[test]
fn inspect_reports_a_wasm_manifest_and_hash() {
    let root = temp_dir("inspect-wasm");
    let dir = root.join("wasm");
    fs::create_dir_all(&dir).unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kawi-plugin/tests/fixtures/wasm/editor-read.wasm");
    fs::copy(fixture, dir.join("plugin.wasm")).unwrap();
    let bytes = fs::read(dir.join("plugin.wasm")).unwrap();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    fs::write(
        dir.join("plugin.toml"),
        format!(
            "name = \"wasm\"\nversion = \"0.1.0\"\nprotocol = 1\nruntime = \"wasm\"\ncomponent = \"plugin.wasm\"\nsha256 = \"{hash}\"\n\n[capabilities]\n\"editor.read\" = true\n"
        ),
    )
    .unwrap();
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
    fs::remove_dir_all(root).unwrap();
}
