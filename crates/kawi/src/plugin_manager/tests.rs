use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use kawi_plugin::PluginLock;

use super::source::PluginSpec;
use super::test_client::{client, client_at, FakeClient};
use super::PluginManager;

static NEXT: AtomicU64 = AtomicU64::new(1);

fn temp_dir(name: &str) -> PathBuf {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "kawi_plugin_manager_{}_{}_{}",
        std::process::id(),
        id,
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn manager(root: &Path) -> PluginManager {
    let mut manager = PluginManager::default();
    manager.configure_for_test(
        root.join("installed"),
        root.join("dev"),
        root.join("plugins.lock"),
    );
    fs::create_dir_all(root.join("installed")).unwrap();
    fs::create_dir_all(root.join("dev")).unwrap();
    manager
}

fn set_client(manager: &mut PluginManager, client: FakeClient) {
    manager.client = Box::new(client);
}

#[test]
fn installs_wasm_plugin_and_writes_lockfile() {
    let root = temp_dir("install");
    let mut manager = manager(&root);
    set_client(&mut manager, client(""));
    let spec = PluginSpec::parse("owner/repo").unwrap();
    let prepared = manager.prepare_install(&spec).unwrap();
    assert_eq!(prepared.name, "echo");
    assert_eq!(prepared.api, 1);
    manager.apply_install(&prepared).unwrap();
    assert!(manager.installed.join("echo/plugin.toml").is_file());
    assert!(manager.installed.join("echo/plugin.wasm").is_file());
    let lock = PluginLock::load(&manager.lock_path).unwrap();
    assert_eq!(lock.plugins["echo"].source, "owner/repo");
    assert_eq!(lock.plugins["echo"].commit, prepared.commit);
    assert!(lock.plugins["echo"].enabled);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn disable_and_remove_update_lockfile_and_files() {
    let root = temp_dir("disable-remove");
    let mut manager = manager(&root);
    set_client(&mut manager, client(""));
    let spec = PluginSpec::parse("owner/repo").unwrap();
    let prepared = manager.prepare_install(&spec).unwrap();
    manager.apply_install(&prepared).unwrap();
    manager.set_enabled("echo", false).unwrap();
    assert!(!manager.lock().plugins["echo"].enabled);
    manager.remove("echo").unwrap();
    assert!(!manager.installed.join("echo").exists());
    assert!(!manager.lock().plugins.contains_key("echo"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn update_detects_added_host_apis() {
    let root = temp_dir("update");
    let mut manager = manager(&root);
    set_client(&mut manager, client(""));
    let spec = PluginSpec::parse("owner/repo").unwrap();
    let prepared = manager.prepare_install(&spec).unwrap();
    manager.apply_install(&prepared).unwrap();
    set_client(&mut manager, client("\"editor.read\" = true\n"));
    let update = manager.prepare_update("echo").unwrap();
    assert!(update.requires_confirmation);
    assert_eq!(update.added, vec![kawi_plugin::Capability::EditorRead]);
    manager.apply_update(&update).unwrap();
    let lock = PluginLock::load(&manager.lock_path).unwrap();
    assert!(lock.plugins["echo"].enabled);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_installed_plugin_from_a_different_source() {
    let root = temp_dir("source-conflict");
    let mut manager = manager(&root);
    set_client(&mut manager, client(""));
    let spec = PluginSpec::parse("owner/repo").unwrap();
    let prepared = manager.prepare_install(&spec).unwrap();
    manager.apply_install(&prepared).unwrap();
    let other = PluginSpec::parse("other/repo").unwrap();
    let error = manager
        .prepare_install(&other)
        .err()
        .expect("source conflict");
    assert!(error.contains("already installed"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn update_reports_current_and_silent_api_changes() {
    let root = temp_dir("update-current");
    let mut manager = manager(&root);
    set_client(&mut manager, client(""));
    let spec = PluginSpec::parse("owner/repo").unwrap();
    let prepared = manager.prepare_install(&spec).unwrap();
    manager.apply_install(&prepared).unwrap();

    let current = manager.prepare_update("echo").unwrap();
    assert!(current.current);
    assert!(!current.requires_confirmation);

    set_client(
        &mut manager,
        client_at("", "1123456789abcdef0123456789abcdef01234567"),
    );
    let changed = manager.prepare_update("echo").unwrap();
    assert!(!changed.current);
    assert!(!changed.requires_confirmation);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn install_rejects_an_existing_plugin() {
    let root = temp_dir("install-existing");
    let mut manager = manager(&root);
    set_client(&mut manager, client(""));
    let spec = PluginSpec::parse("owner/repo").unwrap();
    let prepared = manager.prepare_install(&spec).unwrap();
    manager.apply_install(&prepared).unwrap();
    let error = match manager.prepare_install(&spec) {
        Ok(_) => panic!("install should reject an existing plugin"),
        Err(error) => error,
    };
    assert!(error.contains("use :plugin update"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_lockfile_blocks_mutations() {
    let root = temp_dir("malformed-lock");
    fs::write(root.join("plugins.lock"), "not toml").unwrap();
    let mut manager = manager(&root);
    set_client(&mut manager, client(""));
    let spec = PluginSpec::parse("owner/repo").unwrap();
    let error = manager
        .prepare_install(&spec)
        .err()
        .expect("locked manager");
    assert!(error.contains("cannot modify plugins"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn remove_clears_a_stale_lock_entry_when_files_are_missing() {
    let root = temp_dir("remove-stale");
    let mut manager = manager(&root);
    set_client(&mut manager, client(""));
    let spec = PluginSpec::parse("owner/repo").unwrap();
    let prepared = manager.prepare_install(&spec).unwrap();
    manager.apply_install(&prepared).unwrap();
    fs::remove_dir_all(root.join("installed/echo")).unwrap();
    manager.remove("echo").unwrap();
    assert!(!manager.lock().plugins.contains_key("echo"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_lock_write_restores_the_previous_install() {
    let root = temp_dir("rollback");
    let mut manager = manager(&root);
    set_client(&mut manager, client(""));
    let spec = PluginSpec::parse("owner/repo").unwrap();
    let prepared = manager.prepare_install(&spec).unwrap();
    manager.apply_install(&prepared).unwrap();
    fs::write(root.join("installed/echo/old-marker"), "old").unwrap();

    set_client(
        &mut manager,
        client_at("", "1123456789abcdef0123456789abcdef01234567"),
    );
    let update = manager.prepare_update("echo").unwrap();
    manager.lock_path = root.join("lockdir");
    fs::create_dir_all(&manager.lock_path).unwrap();
    assert!(manager.apply_update(&update).is_err());
    assert!(root.join("installed/echo/old-marker").is_file());
    fs::remove_dir_all(root).unwrap();
}
