use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use kawi_syntax::{registry, SyntaxKey, SyntaxRequest};
use ropey::Rope;
use sha2::{Digest, Sha256};

use super::*;
use crate::test_support::temp_dir;

const ADL_WASM_SHA256: &str = "3ffec313c96b3f2b78a408718403f0e8fb23570c98befb78ee849d9b37093a18";
const ADL_GRAMMAR_REVISION: &str = "5ffcda9174faa0b24331513c00f35cac823c1960";
const ADL_REPOSITORY: &str = "https://github.com/alexytsu/tree-sitter-adl";
const ADL_LICENSE: &str = "MIT";

static GRAMMAR_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn registered_pack_replaces_the_native_queries() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_pack");
    install_and_register(&dir, "(comment) @keyword\n");

    let keyword = registry().index_of("keyword").expect("keyword capture");
    let result = highlight_adl_comment();
    assert!(
        result.spans.iter().any(|span| span.highlight == keyword),
        "expected the pack query to mark the comment as a keyword"
    );

    cleanup(dir);
}

#[test]
fn a_broken_pack_falls_back_to_the_native_grammar() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_broken");
    install_and_register(&dir, "(\n");

    let comment = registry().index_of("comment").expect("comment capture");
    let keyword = registry().index_of("keyword").expect("keyword capture");
    let result = highlight_adl_comment();
    assert!(result.spans.iter().any(|span| span.highlight == comment));
    assert!(!result.spans.iter().any(|span| span.highlight == keyword));

    cleanup(dir);
}

fn lock_grammars() -> MutexGuard<'static, ()> {
    GRAMMAR_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

fn install_and_register(dir: &Path, query: &str) {
    kawi_syntax::clear_external_grammars();
    let source = write_pack(dir, query);
    let user_root = dir.join("user");
    GrammarStore::new(vec![PackRoot::user(&user_root)])
        .install(&source)
        .expect("install pack");
    register(&user_root);
}

fn highlight_adl_comment() -> kawi_syntax::SyntaxResult {
    kawi_syntax::SyntaxService::new().highlight_now(SyntaxRequest {
        key: SyntaxKey {
            document: 1,
            revision: 1,
            top: 0,
            end: 1,
            language: LanguageId::Adl,
        },
        base_revision: 0,
        edits: Vec::new(),
        text: Rope::from_str("// comment\n"),
    })
}

fn cleanup(dir: PathBuf) {
    kawi_syntax::clear_external_grammars();
    fs::remove_dir_all(dir).expect("remove temp dir");
}

fn write_pack(root: &Path, query: &str) -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../kawi-grammar/fixtures/adl/0.7.0");
    let pack = root.join("source");
    fs::create_dir_all(pack.join("queries")).expect("create pack");
    fs::copy(fixture.join("grammar.wasm"), pack.join("grammar.wasm")).expect("copy wasm");
    fs::write(pack.join("queries/highlights.scm"), query).expect("write query");
    let manifest = format!(
        r#"format = 1
id = "adl"
version = "0.7.0"
language = "adl"
grammar = "grammar.wasm"
grammar_sha256 = "{ADL_WASM_SHA256}"
grammar_revision = "{ADL_GRAMMAR_REVISION}"
abi_min = 15
abi_max = 15
minimum_kawi = "0.0.1"
repository = "{ADL_REPOSITORY}"
license = "{ADL_LICENSE}"

[queries]
highlights = "queries/highlights.scm"
highlights_sha256 = "{}"
"#,
        sha256_hex(query.as_bytes())
    );
    fs::write(pack.join("manifest.toml"), manifest).expect("write manifest");
    pack
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
