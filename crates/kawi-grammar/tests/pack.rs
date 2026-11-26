mod support;

use std::fs::{self, OpenOptions};
use std::path::Path;

use kawi_grammar::{GrammarError, Pack, MAX_GRAMMAR_BYTES, MAX_MANIFEST_BYTES};
use sha2::{Digest, Sha256};
use support::{fixture_pack, rewrite_manifest, TempDir};

fn copied_pack(label: &str) -> TempDir {
    let dir = TempDir::new(label);
    support::copy_fixture(dir.path());
    dir
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn opens_the_fixture_pack() {
    let pack = Pack::open(fixture_pack()).expect("open fixture pack");
    assert!(pack.root().is_absolute());
    assert_eq!(pack.manifest().id, "adl");
}

#[test]
fn loads_fixture_contents() {
    let pack = Pack::open(fixture_pack()).expect("open fixture pack");
    let contents = pack.load_contents().expect("load contents");
    assert_eq!(contents.wasm.len(), 28_500);
    assert!(contents.highlights.contains("@keyword"));
    assert!(contents.injections.is_empty());
    assert!(contents.locals.is_empty());
}

#[test]
fn verifies_the_fixture_pack() {
    let pack = Pack::open(fixture_pack()).expect("open fixture pack");
    pack.verify().expect("fixture is valid");
}

#[test]
fn rejects_a_file_as_the_pack_root() {
    let dir = TempDir::new("file-root");
    let path = dir.child("manifest.toml");
    fs::write(&path, "").expect("write file");
    match Pack::open(&path) {
        Err(GrammarError::NotADirectory(_)) => {}
        other => panic!("expected a directory error, got {other:?}"),
    }
}

#[test]
fn rejects_a_missing_manifest() {
    let dir = TempDir::new("missing-manifest");
    match Pack::open(dir.path()) {
        Err(GrammarError::Io { .. }) => {}
        other => panic!("expected an IO error, got {other:?}"),
    }
}

#[test]
fn rejects_an_oversized_manifest() {
    let dir = copied_pack("oversized-manifest");
    let manifest = dir.child("manifest.toml");
    let file = OpenOptions::new()
        .write(true)
        .open(&manifest)
        .expect("open manifest");
    file.set_len(MAX_MANIFEST_BYTES + 1).expect("grow manifest");
    match Pack::open(dir.path()) {
        Err(GrammarError::TooLarge { limit, .. }) => assert_eq!(limit, MAX_MANIFEST_BYTES),
        other => panic!("expected a size error, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn rejects_a_symlink_manifest() {
    use std::os::unix::fs::symlink;

    let dir = TempDir::new("symlink-manifest");
    symlink(Path::new("/etc/hosts"), dir.child("manifest.toml")).expect("create symlink");
    match Pack::open(dir.path()) {
        Err(GrammarError::Symlink(path)) => assert!(path.ends_with("manifest.toml")),
        other => panic!("expected a symlink error, got {other:?}"),
    }
}

#[test]
fn rejects_an_invalid_manifest() {
    let dir = copied_pack("invalid-manifest");
    rewrite_manifest(dir.path(), &[("format = 1", "format = 9")]);
    match Pack::open(dir.path()) {
        Err(GrammarError::UnsupportedFormat { .. }) => {}
        other => panic!("expected a format error, got {other:?}"),
    }
}

#[test]
fn detects_a_modified_grammar() {
    let dir = copied_pack("modified-grammar");
    let wasm = dir.child("grammar.wasm");
    let mut bytes = fs::read(&wasm).expect("read wasm");
    bytes.push(0);
    fs::write(&wasm, bytes).expect("write wasm");
    let pack = Pack::open(dir.path()).expect("open pack");
    match pack.verify() {
        Err(GrammarError::HashMismatch { .. }) => {}
        other => panic!("expected a hash mismatch, got {other:?}"),
    }
}

#[test]
fn detects_a_missing_grammar() {
    let dir = copied_pack("missing-grammar");
    fs::remove_file(dir.child("grammar.wasm")).expect("remove wasm");
    let pack = Pack::open(dir.path()).expect("open pack");
    match pack.load_contents() {
        Err(GrammarError::MissingFile(path)) => assert!(path.ends_with("grammar.wasm")),
        other => panic!("expected a missing file error, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn rejects_a_symlink_that_escapes_the_pack() {
    use std::os::unix::fs::symlink;

    let dir = copied_pack("symlink-escape");
    let wasm = dir.child("grammar.wasm");
    fs::remove_file(&wasm).expect("remove wasm");
    symlink(Path::new("/etc/hosts"), &wasm).expect("create symlink");
    let pack = Pack::open(dir.path()).expect("open pack");
    match pack.load_contents() {
        Err(GrammarError::UnsafePath(path)) => assert_eq!(path, "grammar.wasm"),
        other => panic!("expected an unsafe path error, got {other:?}"),
    }
}

#[test]
fn rejects_an_oversized_grammar() {
    let dir = copied_pack("oversized-grammar");
    let wasm = dir.child("grammar.wasm");
    let file = OpenOptions::new()
        .write(true)
        .open(&wasm)
        .expect("open wasm");
    file.set_len(MAX_GRAMMAR_BYTES + 1).expect("grow wasm");
    let pack = Pack::open(dir.path()).expect("open pack");
    match pack.load_contents() {
        Err(GrammarError::TooLarge { limit, .. }) => assert_eq!(limit, MAX_GRAMMAR_BYTES),
        other => panic!("expected a size error, got {other:?}"),
    }
}

#[test]
fn rejects_an_invalid_utf8_query() {
    let dir = copied_pack("invalid-utf8");
    let query = dir.child("queries/highlights.scm");
    let bytes = [0xff, 0xfe];
    fs::write(&query, bytes).expect("write query");
    let hash = sha256_hex(&bytes);
    rewrite_manifest(
        dir.path(),
        &[(
            "c3e9f89478bc22075a46d7d4518c0984205999348c909eceb06cec7fd4fd005b",
            &hash,
        )],
    );
    let pack = Pack::open(dir.path()).expect("open pack");
    match pack.load_contents() {
        Err(GrammarError::InvalidUtf8 { path, .. }) => {
            assert!(path.ends_with("highlights.scm"))
        }
        other => panic!("expected an invalid UTF-8 error, got {other:?}"),
    }
}

#[test]
fn load_contents_ignores_the_editor_version() {
    let dir = copied_pack("editor_version");
    rewrite_manifest(
        dir.path(),
        &[("minimum_kawi = \"0.0.1\"", "minimum_kawi = \"99.0.0\"")],
    );
    let pack = Pack::open(dir.path()).expect("open pack");
    assert!(pack.load_contents().is_ok());
    assert!(matches!(
        pack.manifest().ensure_host_compatible("0.0.1"),
        Err(GrammarError::UnsupportedKawi { .. })
    ));
}
