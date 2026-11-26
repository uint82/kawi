mod support;

use std::fs;

use kawi_grammar::{GrammarError, GrammarStore, Pack, PackRoot, PackState, PackStatus};
use support::{fixture_pack, install_fixture, TempDir};

fn store(user: &TempDir, bundled: &TempDir, system: &TempDir) -> GrammarStore {
    GrammarStore::new(vec![
        PackRoot::user(user.path()),
        PackRoot::bundled(bundled.path()),
        PackRoot::system(system.path()),
    ])
}

fn status_for<'a>(statuses: &'a [PackStatus], id: &str) -> &'a PackStatus {
    statuses
        .iter()
        .find(|status| status.id == id)
        .unwrap_or_else(|| panic!("missing status for {id}"))
}

fn corrupt_grammar(pack: &std::path::Path) {
    fs::write(pack.join("grammar.wasm"), b"not wasm").expect("corrupt grammar");
}

#[test]
fn discovers_packs_in_every_root() {
    let user = TempDir::new("discover-user");
    let bundled = TempDir::new("discover-bundled");
    let system = TempDir::new("discover-system");
    install_fixture(user.path(), "adl", "0.7.0");
    install_fixture(user.path(), "alpha", "1.0.0");
    install_fixture(bundled.path(), "beta", "2.0.0");
    install_fixture(system.path(), "gamma", "3.0.0");

    let statuses = store(&user, &bundled, &system).discover();
    let ids: Vec<_> = statuses.iter().map(|status| status.id.as_str()).collect();
    assert_eq!(ids, ["adl", "alpha", "beta", "gamma"]);
    assert!(statuses
        .iter()
        .all(|status| matches!(status.state, PackState::Available)));
}

#[test]
fn status_verifies_pack_hashes() {
    let user = TempDir::new("status-user");
    let bundled = TempDir::new("status-bundled");
    let system = TempDir::new("status-system");
    install_fixture(user.path(), "adl", "0.7.0");
    install_fixture(bundled.path(), "alpha", "1.0.0");
    corrupt_grammar(&user.path().join("adl/0.7.0"));

    let statuses = store(&user, &bundled, &system).status();
    assert!(matches!(
        status_for(&statuses, "alpha").state,
        PackState::Valid
    ));
    match &status_for(&statuses, "adl").state {
        PackState::Invalid(reason) => assert!(reason.contains("expected sha256")),
        other => panic!("expected an invalid pack, got {other:?}"),
    }
}

#[test]
fn best_packs_prefers_the_highest_version() {
    let user = TempDir::new("best-user");
    let bundled = TempDir::new("best-bundled");
    let system = TempDir::new("best-system");
    install_fixture(user.path(), "adl", "0.7.0");
    install_fixture(bundled.path(), "adl", "0.8.0");
    install_fixture(system.path(), "adl", "0.9.0");

    let packs = store(&user, &bundled, &system).best_packs();
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].manifest().version, "0.9.0");
    assert!(packs[0]
        .root()
        .starts_with(system.path().canonicalize().unwrap()));
}

#[test]
fn best_packs_prefers_the_user_root_on_equal_versions() {
    let user = TempDir::new("equal-user");
    let bundled = TempDir::new("equal-bundled");
    let system = TempDir::new("equal-system");
    install_fixture(user.path(), "adl", "0.7.0");
    install_fixture(bundled.path(), "adl", "0.7.0");
    install_fixture(system.path(), "adl", "0.7.0");

    let packs = store(&user, &bundled, &system).best_packs();
    assert_eq!(packs.len(), 1);
    assert!(packs[0]
        .root()
        .starts_with(user.path().canonicalize().unwrap()));
}

#[test]
fn best_packs_skips_invalid_packs() {
    let user = TempDir::new("skip-user");
    let bundled = TempDir::new("skip-bundled");
    let system = TempDir::new("skip-system");
    install_fixture(user.path(), "adl", "0.8.0");
    corrupt_grammar(&user.path().join("adl/0.8.0"));
    install_fixture(bundled.path(), "adl", "0.7.0");

    let packs = store(&user, &bundled, &system).best_packs();
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].manifest().version, "0.7.0");
}

#[test]
fn ignores_hidden_directories() {
    let user = TempDir::new("hidden-user");
    let bundled = TempDir::new("hidden-bundled");
    let system = TempDir::new("hidden-system");
    fs::create_dir_all(user.path().join(".staging/adl/0.7.0")).expect("hidden root");
    fs::create_dir_all(user.path().join("adl/.0.7.0")).expect("hidden version");

    assert!(store(&user, &bundled, &system).discover().is_empty());
}

#[test]
fn rejects_a_manifest_that_disagrees_with_its_directory() {
    let user = TempDir::new("mismatch-user");
    let bundled = TempDir::new("mismatch-bundled");
    let system = TempDir::new("mismatch-system");
    support::copy_fixture(&user.path().join("adl/9.9.9"));

    let statuses = store(&user, &bundled, &system).discover();
    assert_eq!(statuses.len(), 1);
    match &statuses[0].state {
        PackState::Invalid(reason) => assert!(reason.contains("manifest declares adl 0.7.0")),
        other => panic!("expected an invalid pack, got {other:?}"),
    }
}

#[test]
fn install_copies_and_verifies_a_pack() {
    let user = TempDir::new("install-user");
    let store = GrammarStore::new(vec![PackRoot::user(user.path())]);

    let installed = store.install(&fixture_pack()).expect("install fixture");
    assert_eq!(installed.id, "adl");
    assert_eq!(installed.version, "0.7.0");
    Pack::open(&installed.path)
        .expect("open installed pack")
        .verify()
        .expect("installed pack verifies");

    let entries: Vec<_> = fs::read_dir(user.path().join("adl"))
        .expect("read id dir")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, ["0.7.0"]);
}

#[test]
fn install_replaces_an_existing_version() {
    let user = TempDir::new("replace-user");
    let store = GrammarStore::new(vec![PackRoot::user(user.path())]);
    store.install(&fixture_pack()).expect("first install");

    let source = TempDir::new("replace-source");
    support::copy_fixture(source.path());
    fs::write(source.path().join("marker"), b"replacement").expect("write marker");
    store.install(source.path()).expect("second install");

    assert!(user.path().join("adl/0.7.0/marker").exists());
    let entries: Vec<_> = fs::read_dir(user.path().join("adl"))
        .expect("read id dir")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, ["0.7.0"]);
}

#[test]
fn install_requires_a_user_root() {
    let bundled = TempDir::new("no-user-root");
    let store = GrammarStore::new(vec![PackRoot::bundled(bundled.path())]);
    match store.install(&fixture_pack()) {
        Err(GrammarError::Install(reason)) => assert!(reason.contains("user grammar root")),
        other => panic!("expected an install error, got {other:?}"),
    }
}

#[test]
fn install_rejects_a_corrupt_source() {
    let user = TempDir::new("corrupt-user");
    let source = TempDir::new("corrupt-source");
    support::copy_fixture(source.path());
    corrupt_grammar(source.path());
    let store = GrammarStore::new(vec![PackRoot::user(user.path())]);

    match store.install(source.path()) {
        Err(GrammarError::HashMismatch { .. }) => {}
        other => panic!("expected a hash mismatch, got {other:?}"),
    }
    assert!(!user.path().join("adl").exists());
}

#[test]
fn install_keeps_the_previous_pack_when_the_new_one_is_bad() {
    let user = TempDir::new("keep-user");
    let store = GrammarStore::new(vec![PackRoot::user(user.path())]);
    store.install(&fixture_pack()).expect("first install");

    let source = TempDir::new("keep-source");
    support::copy_fixture(source.path());
    corrupt_grammar(source.path());
    store
        .install(source.path())
        .expect_err("second install fails");

    Pack::open(user.path().join("adl/0.7.0"))
        .expect("open previous pack")
        .verify()
        .expect("previous pack is intact");
}

#[test]
fn install_rejects_excessive_nesting() {
    let user = TempDir::new("depth-user");
    let source = TempDir::new("depth-source");
    support::copy_fixture(source.path());
    let mut path = source.path().to_path_buf();
    for _ in 0..=16 {
        path.push("nested");
        fs::create_dir(&path).expect("nested directory");
    }
    let store = GrammarStore::new(vec![PackRoot::user(user.path())]);

    match store.install(source.path()) {
        Err(GrammarError::Install(reason)) => assert!(reason.contains("nesting")),
        other => panic!("expected a nesting error, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn install_rejects_symlinks() {
    use std::os::unix::fs::symlink;

    let user = TempDir::new("symlink-user");
    let source = TempDir::new("symlink-source");
    support::copy_fixture(source.path());
    symlink(
        source.path().join("manifest.toml"),
        source.path().join("alias"),
    )
    .expect("create symlink");
    let store = GrammarStore::new(vec![PackRoot::user(user.path())]);

    match store.install(source.path()) {
        Err(GrammarError::Symlink(path)) => assert!(path.ends_with("alias")),
        other => panic!("expected a symlink error, got {other:?}"),
    }
}
