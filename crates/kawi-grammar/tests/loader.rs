mod support;

use std::sync::Arc;

use kawi_grammar::{GrammarError, GrammarLoader, LoadedGrammar, Pack};
use sha2::{Digest, Sha256};
use support::{fixture_pack, rewrite_manifest, TempDir, ADL_SOURCE};

fn loader() -> GrammarLoader {
    GrammarLoader::new().expect("create loader")
}

fn loaded() -> LoadedGrammar {
    let pack = Pack::open(fixture_pack()).expect("open fixture");
    loader().load(&pack).expect("load fixture")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn loader_is_send_and_sync() {
    assert_send_sync::<GrammarLoader>();
    assert_send_sync::<LoadedGrammar>();
}

#[test]
fn loads_the_fixture_pack() {
    let grammar = loaded();
    assert_eq!(grammar.abi_version(), 15);
    assert!(grammar.language().node_kind_count() > 0);
    assert!(grammar.highlights().contains("@keyword"));
    assert!(grammar.injections().is_empty());
    assert!(grammar.locals().is_empty());
}

#[test]
fn parses_adl() {
    let grammar = loaded();
    let mut parser = grammar.parser().expect("create parser");
    let tree = parser.parse(ADL_SOURCE, None).expect("parse");
    assert!(!tree.root_node().has_error());
    assert!(tree.root_node().to_sexp().contains("module_definition"));
}

#[test]
fn creates_independent_parsers() {
    let grammar = loaded();
    let mut first = grammar.parser().expect("first parser");
    let mut second = grammar.parser().expect("second parser");
    let first_tree = first.parse(ADL_SOURCE, None).expect("first parse");
    let second_tree = second.parse(ADL_SOURCE, None).expect("second parse");
    assert_eq!(
        first_tree.root_node().to_sexp(),
        second_tree.root_node().to_sexp()
    );
}

#[test]
fn parses_on_multiple_threads() {
    let grammar = Arc::new(loaded());
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let grammar = Arc::clone(&grammar);
            std::thread::spawn(move || {
                let mut parser = grammar.parser().expect("thread parser");
                let tree = parser.parse(ADL_SOURCE, None).expect("thread parse");
                assert!(!tree.root_node().has_error());
                tree.root_node().to_sexp()
            })
        })
        .collect();
    let sexps: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("thread"))
        .collect();
    assert!(sexps.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn rejects_an_invalid_highlight_query() {
    let dir = TempDir::new("invalid-query");
    support::copy_fixture(dir.path());
    let source = b"(not_a_real_node) @error";
    std::fs::write(dir.path().join("queries/highlights.scm"), source).expect("write query");
    rewrite_manifest(
        dir.path(),
        &[(
            "c3e9f89478bc22075a46d7d4518c0984205999348c909eceb06cec7fd4fd005b",
            &sha256_hex(source),
        )],
    );
    let pack = Pack::open(dir.path()).expect("open pack");
    match loader().load(&pack) {
        Err(GrammarError::Query { kind, .. }) => assert_eq!(kind, "highlights"),
        other => panic!("expected a query error, got {other:?}"),
    }
}

#[test]
fn rejects_a_manifest_abi_that_the_module_does_not_match() {
    let dir = TempDir::new("abi-mismatch");
    support::copy_fixture(dir.path());
    rewrite_manifest(
        dir.path(),
        &[
            ("abi_min = 15", "abi_min = 14"),
            ("abi_max = 15", "abi_max = 14"),
        ],
    );
    let pack = Pack::open(dir.path()).expect("open pack");
    match loader().load(&pack) {
        Err(GrammarError::Abi {
            found: 15,
            min: 14,
            max: 14,
        }) => {}
        other => panic!("expected an ABI error, got {other:?}"),
    }
}

#[test]
fn rejects_a_pack_whose_abi_range_misses_the_host() {
    let dir = TempDir::new("host-abi");
    support::copy_fixture(dir.path());
    rewrite_manifest(
        dir.path(),
        &[
            ("abi_min = 15", "abi_min = 16"),
            ("abi_max = 15", "abi_max = 16"),
        ],
    );
    let pack = Pack::open(dir.path()).expect("open pack");
    match pack.load_contents() {
        Err(GrammarError::AbiRange { .. }) => {}
        other => panic!("expected an ABI range error, got {other:?}"),
    }
}
