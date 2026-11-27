use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use kawi_grammar::PackRoot;
use kawi_syntax::{registry, SyntaxKey, SyntaxRequest};
use ropey::Rope;
use sha2::{Digest, Sha256};

use super::*;
use crate::test_support::temp_dir;

const ADL_WASM_SHA256: &str = "3ffec313c96b3f2b78a408718403f0e8fb23570c98befb78ee849d9b37093a18";
const ADL_GRAMMAR_REVISION: &str = "5ffcda9174faa0b24331513c00f35cac823c1960";
const ADL_REPOSITORY: &str = "https://github.com/alexytsu/tree-sitter-adl";
const ADL_LICENSE: &str = "MIT";
const TYPESCRIPT_WASM_SHA256: &str =
    "36b57857943c4f7a26d877e5588c917a5f0c6eb103a7cdebd1fe146b0c78d7f1";

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
fn registered_bundled_pack_replaces_the_native_queries() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_bundled");
    clear_grammars();
    let bundled_root = dir.join("bundled");
    write_pack(&bundled_root.join("adl/0.7.0"), "(comment) @keyword\n");
    let store = GrammarStore::new(vec![PackRoot::bundled(&bundled_root)]);
    register(&store);

    let keyword = registry().index_of("keyword").expect("keyword capture");
    let result = highlight_adl_comment();
    assert!(
        result.spans.iter().any(|span| span.highlight == keyword),
        "expected the bundled pack query to mark the comment as a keyword"
    );

    cleanup(dir);
}

#[test]
fn user_pack_takes_precedence_over_bundled_pack() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_precedence");
    clear_grammars();
    let user_root = dir.join("user");
    let bundled_root = dir.join("bundled");
    write_pack(&bundled_root.join("adl/0.7.0"), "(comment) @string\n");
    let source = write_pack(&dir.join("source"), "(comment) @keyword\n");
    let store = GrammarStore::new(vec![
        PackRoot::user(&user_root),
        PackRoot::bundled(&bundled_root),
    ]);
    store.install(&source).expect("install user pack");
    register(&store);

    let keyword = registry().index_of("keyword").expect("keyword capture");
    let string = registry().index_of("string").expect("string capture");
    let result = highlight_adl_comment();
    assert!(result.spans.iter().any(|span| span.highlight == keyword));
    assert!(!result.spans.iter().any(|span| span.highlight == string));

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

#[test]
fn a_broken_typescript_pack_falls_back_to_native() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_typescript_broken");
    clear_grammars();
    let source = dir.join("source");
    copy_typescript_pack(&source);
    let broken = b"not wasm";
    fs::write(source.join("grammar.wasm"), broken).expect("write broken wasm");
    replace_manifest_hash(&source, &sha256_hex(broken));

    let user_root = dir.join("user");
    let store = GrammarStore::new(vec![PackRoot::user(&user_root)]);
    store.install(&source).expect("install pack");
    register(&store);

    let keyword = registry().index_of("keyword").expect("keyword capture");
    let result = highlight_typescript();
    assert!(
        result.spans.iter().any(|span| span.highlight == keyword),
        "expected the native TypeScript grammar to highlight a keyword"
    );

    cleanup(dir);
}

fn lock_grammars() -> MutexGuard<'static, ()> {
    GRAMMAR_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

fn install_and_register(dir: &Path, query: &str) {
    clear_grammars();
    let source = write_pack(&dir.join("source"), query);
    let user_root = dir.join("user");
    let store = GrammarStore::new(vec![PackRoot::user(&user_root)]);
    store.install(&source).expect("install pack");
    register(&store);
}

fn clear_grammars() {
    kawi_syntax::clear_external_grammars();
}

fn highlight_typescript() -> kawi_syntax::SyntaxResult {
    kawi_syntax::SyntaxService::new().highlight_now(SyntaxRequest {
        key: SyntaxKey {
            document: 1,
            revision: 1,
            top: 0,
            end: 1,
            language: LanguageId::TypeScript,
        },
        base_revision: 0,
        edits: Vec::new(),
        text: Rope::from_str("const value: number = 1;\n"),
    })
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
    clear_grammars();
    fs::remove_dir_all(dir).expect("remove temp dir");
}

fn write_pack(pack: &Path, query: &str) -> PathBuf {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../kawi-grammar/packs/adl/0.7.0");
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
    pack.to_path_buf()
}

fn copy_typescript_pack(target: &Path) {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../kawi-grammar/packs/typescript/0.23.2");
    copy_tree(&source, target);
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("create pack");
    for entry in fs::read_dir(source).expect("read pack") {
        let entry = entry.expect("pack entry");
        let destination = target.join(entry.file_name());
        if entry.file_type().expect("pack entry type").is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), &destination).expect("copy pack file");
        }
    }
}

fn replace_manifest_hash(pack: &Path, hash: &str) {
    let path = pack.join("manifest.toml");
    let text = fs::read_to_string(&path).expect("read manifest");
    fs::write(&path, text.replace(TYPESCRIPT_WASM_SHA256, hash)).expect("write manifest");
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
