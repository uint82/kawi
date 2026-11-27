#![allow(dead_code)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use kawi_grammar::LoadedGrammar;
use tree_sitter::{
    InputEdit, Language, Parser, Point, Query, QueryCursor, StreamingIterator, Tree,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub const ADL_SOURCE: &str = r#"module example {

import foo.bar.*

/// A person.
struct Person {
  String name
  Int32 age = 0
}

union Shape {
  Double circle
  Double square
}

newtype UserId = String = "none"

type Alias = Person

annotation Person::name String "renamed"
}
"#;

pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!("kawi-grammar-{label}-{}", unique_suffix()));
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn child(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn fixture_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("packs")
        .join("adl")
        .join("0.7.0")
}

pub fn fixture_typescript_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("packs")
        .join("typescript")
        .join("0.23.2")
}

pub fn fixture_markdown_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("packs")
        .join("markdown")
        .join("0.5.4")
}

pub fn fixture_markdown_inline_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("packs")
        .join("markdown_inline")
        .join("0.5.4")
}

pub fn fixture_manifest_text() -> String {
    fs::read_to_string(fixture_pack().join("manifest.toml")).expect("read fixture manifest")
}

pub fn copy_fixture(target: &Path) {
    copy_dir(&fixture_pack(), target).expect("copy fixture pack");
}

pub fn install_fixture(root: &Path, id: &str, version: &str) {
    let target = root.join(id).join(version);
    copy_fixture(&target);
    let id_line = format!("id = \"{id}\"");
    let version_line = format!("version = \"{version}\"");
    rewrite_manifest(
        &target,
        &[
            ("id = \"adl\"", &id_line),
            ("version = \"0.7.0\"", &version_line),
        ],
    );
}

pub fn rewrite_manifest(root: &Path, replacements: &[(&str, &str)]) {
    let path = root.join("manifest.toml");
    let mut text = fs::read_to_string(&path).expect("read manifest");
    for (from, to) in replacements {
        assert!(text.contains(from), "manifest does not contain {from:?}");
        text = text.replace(from, to);
    }
    fs::write(&path, text).expect("write manifest");
}

pub fn parse_with(language: Language, source: &str) -> Tree {
    let mut parser = Parser::new();
    parser.set_language(&language).expect("set native language");
    parser.parse(source, None).expect("parse source")
}

pub fn parse_loaded(grammar: &LoadedGrammar, source: &str) -> Tree {
    let mut parser = grammar.parser().expect("create parser");
    parser.parse(source, None).expect("parse source")
}

pub fn captures(
    language: &Language,
    tree: &Tree,
    query_source: &str,
    source: &str,
) -> Vec<(String, usize, usize)> {
    let query = Query::new(language, query_source).expect("compile query");
    let names = query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut result = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(matched) = matches.next() {
        for capture in matched.captures() {
            result.push((
                names[capture.index as usize].to_string(),
                capture.node.start_byte(),
                capture.node.end_byte(),
            ));
        }
    }
    result
}

pub fn point_at(source: &str, offset: usize) -> Point {
    let mut point = Point::new(0, 0);
    for byte in source.bytes().take(offset) {
        if byte == b'\n' {
            point.row += 1;
            point.column = 0;
        } else {
            point.column += 1;
        }
    }
    point
}

pub fn assert_incremental_parity(
    grammar: &LoadedGrammar,
    native: Language,
    source: &str,
    updated: &str,
    edit: InputEdit,
) {
    let mut native_parser = Parser::new();
    native_parser
        .set_language(&native)
        .expect("set native language");
    let mut wasm_parser = grammar.parser().expect("create wasm parser");

    let mut native_tree = native_parser.parse(source, None).expect("native parse");
    let mut wasm_tree = wasm_parser.parse(source, None).expect("wasm parse");

    native_tree.edit(&edit);
    wasm_tree.edit(&edit);
    let native_after = native_parser
        .parse(updated, Some(&native_tree))
        .expect("native reparse");
    let wasm_after = wasm_parser
        .parse(updated, Some(&wasm_tree))
        .expect("wasm reparse");

    assert_eq!(
        native_after.root_node().to_sexp(),
        wasm_after.root_node().to_sexp()
    );
}

pub fn large_typescript_source(functions: usize) -> String {
    use std::fmt::Write;

    let mut source = String::with_capacity(functions * 96);
    for index in 0..functions {
        writeln!(
            source,
            "export function fn{index}(value: number): number {{"
        )
        .expect("write source");
        writeln!(source, "  const next = value + {index};").expect("write source");
        source.push_str("  return next * 2;\n}\n");
    }
    source
}

fn copy_dir(source: &Path, target: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&path, &destination)?;
        } else {
            fs::copy(&path, &destination)?;
        }
    }
    Ok(())
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{nanos}-{counter}", std::process::id())
}
