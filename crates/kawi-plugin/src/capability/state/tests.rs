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
    catalog_with(
        root,
        "name = \"sample\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"/bin/true\"]\n\n[capabilities]\n\"editor.read\" = true\n\"ui.message\" = true\n",
    )
}

fn catalog_with(root: &Path, manifest: &str) -> crate::manifest::PluginCatalog {
    let plugin = root.join("sample");
    fs::create_dir_all(&plugin).unwrap();
    fs::write(plugin.join("plugin.toml"), manifest).unwrap();
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

#[test]
fn warns_when_subscription_capability_is_missing() {
    let root = temp_dir();
    let manifest = "name = \"sample\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [\"/bin/true\"]\nsubscribes = [\"buffer_changed\", \"diagnostics_changed\"]\n";
    let state = load_permission_state(
        &catalog_with(&root, manifest),
        &root.join("permissions.toml"),
    );
    assert!(state
        .warnings
        .iter()
        .any(|warning| warning.contains("subscription buffer_changed requires editor.read")));
    assert!(state.warnings.iter().any(|warning| {
        warning.contains("subscription diagnostics_changed requires diagnostics.read")
    }));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn warns_when_permissions_file_is_writable_by_others() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_dir();
    let path = root.join("permissions.toml");
    fs::write(&path, "").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
    let state = load_permission_state(&catalog(&root), &path);
    assert!(state
        .warnings
        .iter()
        .any(|warning| warning.contains("writable by group or other users")));
    fs::remove_dir_all(root).unwrap();
}
