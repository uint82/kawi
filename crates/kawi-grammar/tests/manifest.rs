mod support;

use std::fs;
use std::path::PathBuf;

use kawi_grammar::{GrammarError, Manifest};
use support::{fixture_pack, rewrite_manifest, TempDir};

fn load_with(replacements: &[(&str, &str)]) -> Result<Manifest, GrammarError> {
    let dir = TempDir::new("manifest");
    support::copy_fixture(dir.path());
    rewrite_manifest(dir.path(), replacements);
    Manifest::load(&dir.path().join("manifest.toml"))
}

fn fixture_manifest() -> Manifest {
    Manifest::load(&fixture_pack().join("manifest.toml")).expect("valid fixture manifest")
}

fn assert_invalid(result: Result<Manifest, GrammarError>) {
    match result {
        Err(GrammarError::InvalidManifest(_) | GrammarError::Manifest { .. }) => {}
        other => panic!("expected an invalid manifest, got {other:?}"),
    }
}

#[test]
fn loads_the_fixture_manifest() {
    let manifest = fixture_manifest();
    assert_eq!(manifest.id, "adl");
    assert_eq!(manifest.version, "0.7.0");
    assert_eq!(manifest.language, "adl");
    assert_eq!(manifest.abi_min, 15);
    assert_eq!(manifest.abi_max, 15);
    assert_eq!(
        manifest.queries.highlights,
        PathBuf::from("queries/highlights.scm")
    );
}

#[test]
fn rejects_unknown_top_level_fields() {
    assert_invalid(load_with(&[("[queries]", "surprise = true\n\n[queries]")]));
}

#[test]
fn rejects_unknown_query_fields() {
    assert_invalid(load_with(&[(
        "highlights = \"queries/highlights.scm\"",
        "highlights = \"queries/highlights.scm\"\nsurprise = true",
    )]));
}

#[test]
fn rejects_an_unsupported_format() {
    match load_with(&[("format = 1", "format = 2")]) {
        Err(GrammarError::UnsupportedFormat {
            found: 2,
            supported: 1,
        }) => {}
        other => panic!("expected an unsupported format, got {other:?}"),
    }
}

#[test]
fn rejects_an_invalid_id() {
    assert_invalid(load_with(&[("id = \"adl\"", "id = \"ADL\"")]));
    assert_invalid(load_with(&[("id = \"adl\"", "id = \"../adl\"")]));
}

#[test]
fn rejects_an_invalid_version() {
    assert_invalid(load_with(&[("version = \"0.7.0\"", "version = \"0.7\"")]));
}

#[test]
fn rejects_an_invalid_language_id() {
    assert_invalid(load_with(&[("language = \"adl\"", "language = \"ADL\"")]));
}

#[test]
fn rejects_an_absolute_grammar_path() {
    assert_invalid(load_with(&[(
        "grammar = \"grammar.wasm\"",
        "grammar = \"/tmp/grammar.wasm\"",
    )]));
}

#[test]
fn rejects_parent_traversal() {
    assert_invalid(load_with(&[(
        "grammar = \"grammar.wasm\"",
        "grammar = \"../grammar.wasm\"",
    )]));
}

#[test]
fn rejects_backslash_paths() {
    assert_invalid(load_with(&[(
        "grammar = \"grammar.wasm\"",
        "grammar = \"queries\\\\grammar.wasm\"",
    )]));
}

#[test]
fn rejects_a_malformed_hash() {
    assert_invalid(load_with(&[(
        "grammar_sha256 = \"",
        "grammar_sha256 = \"xyz",
    )]));
}

#[test]
fn rejects_abi_min_above_abi_max() {
    assert_invalid(load_with(&[("abi_min = 15", "abi_min = 16")]));
}

#[test]
fn rejects_a_zero_abi_min() {
    assert_invalid(load_with(&[("abi_min = 15", "abi_min = 0")]));
}

#[test]
fn rejects_an_injections_path_without_a_hash() {
    assert_invalid(load_with(&[(
        "highlights_sha256 = \"c3e9f89478bc22075a46d7d4518c0984205999348c909eceb06cec7fd4fd005b\"",
        "highlights_sha256 = \"c3e9f89478bc22075a46d7d4518c0984205999348c909eceb06cec7fd4fd005b\"\ninjections = \"queries/injections.scm\"",
    )]));
}

#[test]
fn rejects_an_injections_hash_without_a_path() {
    assert_invalid(load_with(&[(
        "highlights_sha256 = \"c3e9f89478bc22075a46d7d4518c0984205999348c909eceb06cec7fd4fd005b\"",
        "highlights_sha256 = \"c3e9f89478bc22075a46d7d4518c0984205999348c909eceb06cec7fd4fd005b\"\ninjections_sha256 = \"c3e9f89478bc22075a46d7d4518c0984205999348c909eceb06cec7fd4fd005b\"",
    )]));
}

#[test]
fn rejects_a_locals_path_without_a_hash() {
    assert_invalid(load_with(&[(
        "highlights_sha256 = \"c3e9f89478bc22075a46d7d4518c0984205999348c909eceb06cec7fd4fd005b\"",
        "highlights_sha256 = \"c3e9f89478bc22075a46d7d4518c0984205999348c909eceb06cec7fd4fd005b\"\nlocals = \"queries/locals.scm\"",
    )]));
}

#[test]
fn rejects_an_empty_repository() {
    assert_invalid(load_with(&[(
        "repository = \"https://github.com/alexytsu/tree-sitter-adl\"",
        "repository = \"\"",
    )]));
}

#[test]
fn rejects_control_characters_in_text_fields() {
    assert_invalid(load_with(&[(
        "license = \"MIT\"",
        "license = \"MIT\\u0007\"",
    )]));
}

#[test]
fn rejects_a_manifest_that_is_not_utf8() {
    let dir = TempDir::new("manifest-utf8");
    fs::write(dir.child("manifest.toml"), [0xff, 0xfe]).expect("write manifest");
    match Manifest::load(&dir.child("manifest.toml")) {
        Err(GrammarError::InvalidUtf8 { path, .. }) => assert!(path.ends_with("manifest.toml")),
        other => panic!("expected an invalid UTF-8 error, got {other:?}"),
    }
}

#[test]
fn rejects_a_pack_that_needs_a_newer_kawi() {
    let manifest = load_with(&[("minimum_kawi = \"0.0.1\"", "minimum_kawi = \"99.0.0\"")])
        .expect("manifest is structurally valid");
    match manifest.ensure_compatible("0.0.1", 13, 15) {
        Err(GrammarError::UnsupportedKawi { required, current }) => {
            assert_eq!(required, "99.0.0");
            assert_eq!(current, "0.0.1");
        }
        other => panic!("expected an unsupported kawi version, got {other:?}"),
    }
}

#[test]
fn rejects_an_incompatible_abi_range() {
    let manifest = fixture_manifest();
    match manifest.ensure_compatible("0.0.1", 16, 18) {
        Err(GrammarError::AbiRange {
            pack_min: 15,
            pack_max: 15,
            host_min: 16,
            host_max: 18,
        }) => {}
        other => panic!("expected an ABI range error, got {other:?}"),
    }
}

#[test]
fn accepts_a_compatible_abi_range() {
    let manifest = fixture_manifest();
    manifest
        .ensure_compatible("0.0.1", 13, 15)
        .expect("host range overlaps the pack range");
}

#[test]
fn accepts_the_declared_abi() {
    let manifest = fixture_manifest();
    manifest.ensure_abi(15).expect("pack declares ABI 15");
}

#[test]
fn rejects_an_abi_outside_the_manifest_range() {
    let manifest = fixture_manifest();
    match manifest.ensure_abi(14) {
        Err(GrammarError::Abi {
            found: 14,
            min: 15,
            max: 15,
        }) => {}
        other => panic!("expected an ABI error, got {other:?}"),
    }
}
