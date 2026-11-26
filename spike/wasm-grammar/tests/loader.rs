mod support;

use kawi_grammar_spike::{sha256_hex, GrammarEngine, GrammarSpec, LoadError};

fn engine() -> GrammarEngine {
    GrammarEngine::new().expect("Wasm engine")
}

#[test]
fn loads_the_typescript_grammar() {
    let grammar = engine()
        .load(&support::TYPESCRIPT, support::TYPESCRIPT_WASM)
        .expect("load TypeScript grammar");

    assert!(grammar.is_wasm());
    assert_eq!(grammar.abi_version(), support::TYPESCRIPT_ABI);
    assert!(grammar.language().node_kind_count() > 0);
}

#[test]
fn fixture_hash_matches_provenance() {
    assert_eq!(
        sha256_hex(support::TYPESCRIPT_WASM),
        support::TYPESCRIPT_SHA256
    );
}

#[test]
fn accepts_an_uppercase_hash() {
    let sha256 = support::TYPESCRIPT_SHA256.to_uppercase();
    let spec = GrammarSpec {
        name: "typescript",
        sha256: &sha256,
    };

    assert!(engine().load(&spec, support::TYPESCRIPT_WASM).is_ok());
}

#[test]
fn rejects_a_hash_mismatch() {
    let sha256 = "0".repeat(64);
    let spec = GrammarSpec {
        name: "typescript",
        sha256: &sha256,
    };
    let error = engine()
        .load(&spec, support::TYPESCRIPT_WASM)
        .expect_err("hash mismatch");

    assert!(matches!(error, LoadError::Hash(_)));
}

#[test]
fn rejects_an_invalid_hash_format() {
    let spec = GrammarSpec {
        name: "typescript",
        sha256: "not-a-hash",
    };
    let error = engine()
        .load(&spec, support::TYPESCRIPT_WASM)
        .expect_err("invalid hash");

    assert!(matches!(error, LoadError::Hash(_)));
}

#[test]
fn rejects_empty_wasm() {
    let sha256 = sha256_hex(b"");
    let spec = GrammarSpec {
        name: "typescript",
        sha256: &sha256,
    };
    let error = engine().load(&spec, b"").expect_err("empty module");

    assert!(matches!(error, LoadError::Wasm(_)));
}

#[test]
fn rejects_truncated_wasm() {
    let truncated = &support::TYPESCRIPT_WASM[..support::TYPESCRIPT_WASM.len() / 2];
    let sha256 = sha256_hex(truncated);
    let spec = GrammarSpec {
        name: "typescript",
        sha256: &sha256,
    };
    let error = engine()
        .load(&spec, truncated)
        .expect_err("truncated module");

    assert!(matches!(error, LoadError::Wasm(_)));
}

#[test]
fn rejects_the_wrong_grammar_name() {
    let spec = GrammarSpec {
        name: "javascript",
        sha256: support::TYPESCRIPT_SHA256,
    };
    let error = engine()
        .load(&spec, support::TYPESCRIPT_WASM)
        .expect_err("wrong grammar name");

    assert!(matches!(error, LoadError::Wasm(_)));
}

#[test]
fn rejects_an_empty_grammar_name() {
    let spec = GrammarSpec {
        name: "",
        sha256: support::TYPESCRIPT_SHA256,
    };
    let error = engine()
        .load(&spec, support::TYPESCRIPT_WASM)
        .expect_err("empty grammar name");

    assert!(matches!(error, LoadError::Wasm(_)));
}
