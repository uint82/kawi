#![allow(dead_code)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tree_sitter::{Language, Parser, Tree};

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
        .join("fixtures")
        .join("adl")
        .join("0.7.0")
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
