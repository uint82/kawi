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
const MARKDOWN_WASM_SHA256: &str =
    "474ad6d0f6e66b641c89d39a9f8afeb64f67be7a3a79679d82168e57a806c1c1";
const MARKDOWN_INLINE_WASM_SHA256: &str =
    "4426bd0aa98c7effbf21c12f4c2cb86fb03e9ef3931d1f6d8782f55d60fe7d63";
const HTML_WASM_SHA256: &str = "91941c44a17a53939c79532214531877c816fa6d7397564d88ce34612d4d25d6";
const HTML_HIGHLIGHTS_SHA256: &str =
    "5f1c18aea4b1302362d2738caab30f7f0f2c60ffde652b8dde52cd43518adac1";

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
    copy_pack("typescript/0.23.2", &source);
    let broken = b"not wasm";
    fs::write(source.join("grammar.wasm"), broken).expect("write broken wasm");
    replace_manifest_hash(&source, TYPESCRIPT_WASM_SHA256, &sha256_hex(broken));

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

#[test]
fn bundled_markdown_packs_highlight_injections() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_markdown");
    clear_grammars();
    let bundled_root = dir.join("bundled");
    copy_pack("markdown/0.5.4", &bundled_root.join("markdown/0.5.4"));
    copy_pack(
        "markdown_inline/0.5.4",
        &bundled_root.join("markdown_inline/0.5.4"),
    );
    let store = GrammarStore::new(vec![PackRoot::bundled(&bundled_root)]);
    register(&store);

    let keyword = registry()
        .index_of("keyword.function")
        .expect("keyword.function capture");
    let italic = registry()
        .index_of("markup.italic")
        .expect("markup.italic capture");
    let tag = registry().index_of("tag").expect("tag capture");

    let fenced = highlight_markdown("```rust\nfn main() {}\n```\n");
    assert!(
        fenced.spans.iter().any(|span| span.highlight == keyword),
        "expected the Rust injection to highlight a keyword"
    );

    let inline = highlight_markdown("Some *italic* text.\n");
    assert!(
        inline.spans.iter().any(|span| span.highlight == italic),
        "expected the inline injection to highlight italic text"
    );

    let html = highlight_markdown("Some <span>inside</span> tail.\n");
    assert!(
        html.spans.iter().any(|span| span.highlight == tag),
        "expected the HTML injection to highlight a tag"
    );

    cleanup(dir);
}

#[test]
fn a_broken_markdown_pack_falls_back_to_native() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_markdown_broken");
    clear_grammars();
    let source = dir.join("source");
    copy_pack("markdown/0.5.4", &source);
    let broken = b"not wasm";
    fs::write(source.join("grammar.wasm"), broken).expect("write broken wasm");
    replace_manifest_hash(&source, MARKDOWN_WASM_SHA256, &sha256_hex(broken));

    let user_root = dir.join("user");
    let store = GrammarStore::new(vec![PackRoot::user(&user_root)]);
    store.install(&source).expect("install pack");
    register(&store);

    let heading = registry()
        .index_of("markup.heading.1")
        .expect("markup.heading.1 capture");
    let result = highlight_markdown("# Heading\n");
    assert!(
        result.spans.iter().any(|span| span.highlight == heading),
        "expected the native Markdown grammar to highlight a heading"
    );

    cleanup(dir);
}

#[test]
fn a_broken_markdown_inline_pack_falls_back_to_native() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_markdown_inline_broken");
    clear_grammars();
    let bundled_root = dir.join("bundled");
    copy_pack("markdown/0.5.4", &bundled_root.join("markdown/0.5.4"));
    let source = dir.join("source");
    copy_pack("markdown_inline/0.5.4", &source);
    let broken = b"not wasm";
    fs::write(source.join("grammar.wasm"), broken).expect("write broken wasm");
    replace_manifest_hash(&source, MARKDOWN_INLINE_WASM_SHA256, &sha256_hex(broken));

    let user_root = dir.join("user");
    let store = GrammarStore::new(vec![
        PackRoot::user(&user_root),
        PackRoot::bundled(&bundled_root),
    ]);
    store.install(&source).expect("install pack");
    register(&store);

    let italic = registry()
        .index_of("markup.italic")
        .expect("markup.italic capture");
    let result = highlight_markdown("Some *italic* text.\n");
    assert!(
        result.spans.iter().any(|span| span.highlight == italic),
        "expected the native Markdown inline grammar to highlight italic text"
    );

    cleanup(dir);
}

#[test]
fn registered_html_pack_highlights_script_and_style_injections() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_html");
    clear_grammars();
    let bundled_root = dir.join("bundled");
    copy_html_pack_with_query(&bundled_root.join("html/0.23.2"), "(tag_name) @comment\n");
    let store = GrammarStore::new(vec![PackRoot::bundled(&bundled_root)]);
    register(&store);

    let comment = registry().index_of("comment").expect("comment capture");
    let keyword = registry().index_of("keyword").expect("keyword capture");
    let property = registry().index_of("property").expect("property capture");

    let tag = highlight_html("<div></div>\n");
    assert!(
        tag.spans.iter().any(|span| span.highlight == comment),
        "expected the HTML pack query to mark a tag as a comment"
    );

    let script = highlight_html("<script>const value = 1;</script>\n");
    assert!(
        script.spans.iter().any(|span| span.highlight == keyword),
        "expected the JavaScript injection to highlight a keyword"
    );

    let style = highlight_html("<style>.box { color: red; }</style>\n");
    assert!(
        style.spans.iter().any(|span| span.highlight == property),
        "expected the CSS injection to highlight a property"
    );

    cleanup(dir);
}

#[test]
fn markdown_injects_into_the_bundled_html_pack() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_markdown_html");
    clear_grammars();
    let bundled_root = dir.join("bundled");
    copy_pack("markdown/0.5.4", &bundled_root.join("markdown/0.5.4"));
    copy_pack(
        "markdown_inline/0.5.4",
        &bundled_root.join("markdown_inline/0.5.4"),
    );
    copy_html_pack_with_query(&bundled_root.join("html/0.23.2"), "(tag_name) @comment\n");
    let store = GrammarStore::new(vec![PackRoot::bundled(&bundled_root)]);
    register(&store);

    let comment = registry().index_of("comment").expect("comment capture");
    let result = highlight_markdown("Some <span>inside</span> tail.\n");
    assert!(
        result.spans.iter().any(|span| span.highlight == comment),
        "expected Markdown inline HTML to use the HTML pack query"
    );

    cleanup(dir);
}

#[test]
fn a_broken_html_pack_falls_back_to_native() {
    let _guard = lock_grammars();
    let dir = temp_dir("grammar_html_broken");
    clear_grammars();
    let source = dir.join("source");
    copy_pack("html/0.23.2", &source);
    let broken = b"not wasm";
    fs::write(source.join("grammar.wasm"), broken).expect("write broken wasm");
    replace_manifest_hash(&source, HTML_WASM_SHA256, &sha256_hex(broken));

    let user_root = dir.join("user");
    let store = GrammarStore::new(vec![PackRoot::user(&user_root)]);
    store.install(&source).expect("install pack");
    register(&store);

    let tag = registry().index_of("tag").expect("tag capture");
    let comment = registry().index_of("comment").expect("comment capture");
    let result = highlight_html("<div></div>\n");
    assert!(
        result.spans.iter().any(|span| span.highlight == tag),
        "expected the native HTML grammar to highlight a tag"
    );
    assert!(
        !result.spans.iter().any(|span| span.highlight == comment),
        "expected the broken HTML pack not to be used"
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
    highlight(LanguageId::TypeScript, "const value: number = 1;\n")
}

fn highlight_markdown(text: &str) -> kawi_syntax::SyntaxResult {
    highlight(LanguageId::Markdown, text)
}

fn highlight_html(text: &str) -> kawi_syntax::SyntaxResult {
    highlight(LanguageId::Html, text)
}

fn highlight_adl_comment() -> kawi_syntax::SyntaxResult {
    highlight(LanguageId::Adl, "// comment\n")
}

fn highlight(language: LanguageId, text: &str) -> kawi_syntax::SyntaxResult {
    kawi_syntax::SyntaxService::new().highlight_now(SyntaxRequest {
        key: SyntaxKey {
            document: 1,
            revision: 1,
            top: 0,
            end: text.lines().count().max(1),
            language,
        },
        base_revision: 0,
        edits: Vec::new(),
        text: Rope::from_str(text),
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

fn copy_pack(pack: &str, target: &Path) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kawi-grammar/packs")
        .join(pack);
    copy_tree(&source, target);
}

fn copy_html_pack_with_query(target: &Path, query: &str) {
    copy_pack("html/0.23.2", target);
    fs::write(target.join("queries/highlights.scm"), query).expect("write query");
    replace_manifest_hash(
        target,
        HTML_HIGHLIGHTS_SHA256,
        &sha256_hex(query.as_bytes()),
    );
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

fn replace_manifest_hash(pack: &Path, from: &str, to: &str) {
    let path = pack.join("manifest.toml");
    let text = fs::read_to_string(&path).expect("read manifest");
    fs::write(&path, text.replace(from, to)).expect("write manifest");
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
