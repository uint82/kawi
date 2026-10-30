use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::capability::Capability;
use crate::manifest::load_catalog;

use super::*;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("kawi-permission-state-{}-{id}", std::process::id()));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn catalog(root: &Path) -> crate::manifest::PluginCatalog {
    let plugin = root.join("sample");
    fs::create_dir_all(&plugin).unwrap();
    fs::write(
        plugin.join("plugin.toml"),
        "name = \"sample\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"/bin/true\"]\n\n[capabilities]\n\"editor.read\" = true\n\"ui.message\" = true\n",
    )
    .unwrap();
    load_catalog(root)
}

#[test]
fn missing_permissions_deny_requested_capabilities() {
    let root = temp_dir();
    let state = load_permission_state(&catalog(&root), &root.join("permissions.toml"));
    assert!(!state.grants["sample"].has(Capability::EditorRead));
    assert!(state.grants["sample"].has(Capability::UiMessage));
    assert!(state
        .warnings
        .iter()
        .any(|warning| warning.contains("editor.read")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_permissions_fail_closed() {
    let root = temp_dir();
    let path = root.join("permissions.toml");
    fs::write(&path, "not toml").unwrap();
    let state = load_permission_state(&catalog(&root), &path);
    assert!(state.file_error.is_some());
    assert!(!state.grants["sample"].has(Capability::EditorRead));
    assert!(state.grants["sample"].has(Capability::UiMessage));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn grants_narrow_requested_capabilities() {
    let root = temp_dir();
    let path = root.join("permissions.toml");
    fs::write(
        &path,
        "[plugins.sample.approved]\n\"editor.read\" = true\n\"ui.message\" = true\n\n[plugins.sample.grant]\n\"editor.read\" = true\n",
    )
    .unwrap();
    let state = load_permission_state(&catalog(&root), &path);
    assert!(state.grants["sample"].has(Capability::EditorRead));
    assert!(state.file_error.is_none());
    fs::remove_dir_all(root).unwrap();
}
