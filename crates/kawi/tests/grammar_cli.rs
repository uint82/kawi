#![cfg(feature = "wasm-grammars")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Workspace {
    root: PathBuf,
    binary: PathBuf,
}

impl Workspace {
    fn new(name: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "kawi_grammar_cli_{}_{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_dir_all(&root);
        for directory in ["data", "config", "state", "cache", "bin"] {
            fs::create_dir_all(root.join(directory)).expect("workspace directory");
        }
        let binary = root.join("bin").join(binary_name());
        fs::hard_link(env!("CARGO_BIN_EXE_kawi"), &binary)
            .or_else(|_| fs::copy(env!("CARGO_BIN_EXE_kawi"), &binary).map(|_| ()))
            .expect("place kawi binary");
        Self { root, binary }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("XDG_CACHE_HOME", self.root.join("cache"));
        command
    }

    fn bundled_root(&self) -> PathBuf {
        self.root.join("share/kawi/grammars")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "kawi.exe"
    } else {
        "kawi"
    }
}

fn fixture_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../kawi-grammar/packs/adl/0.7.0")
}

fn grammar(workspace: &Workspace, args: &[&str]) -> Output {
    workspace
        .command()
        .arg("grammar")
        .args(args)
        .output()
        .expect("run kawi grammar")
}

fn stdout(output: Output) -> String {
    String::from_utf8(output.stdout).expect("utf-8 output")
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("create target");
    for entry in fs::read_dir(source).expect("read source") {
        let entry = entry.expect("source entry");
        let path = entry.path();
        let destination = target.join(entry.file_name());
        if entry.file_type().expect("source type").is_dir() {
            copy_tree(&path, &destination);
        } else {
            fs::copy(&path, &destination).expect("copy file");
        }
    }
}

#[test]
fn installs_and_reports_a_pack() {
    let workspace = Workspace::new("install");
    let empty = grammar(&workspace, &["status"]);
    assert!(empty.status.success(), "{:?}", empty.stderr);
    assert!(stdout(empty).contains("no grammar packs installed"));

    let install = grammar(
        &workspace,
        &["install", fixture_pack().to_str().expect("fixture path")],
    );
    assert!(install.status.success(), "{:?}", install.stderr);
    assert!(stdout(install).contains("installed adl 0.7.0"));

    let status = grammar(&workspace, &["status"]);
    assert!(status.status.success(), "{:?}", status.stderr);
    assert!(stdout(status).contains("adl 0.7.0 [user] valid"));
}

#[test]
fn reports_a_bundled_pack_next_to_bin() {
    let workspace = Workspace::new("bundled");
    copy_tree(&fixture_pack(), &workspace.bundled_root().join("adl/0.7.0"));

    let status = grammar(&workspace, &["status"]);
    assert!(status.status.success(), "{:?}", status.stderr);
    assert!(stdout(status).contains("adl 0.7.0 [bundled] valid"));
}

#[test]
fn rejects_a_missing_pack() {
    let workspace = Workspace::new("missing");
    let install = grammar(&workspace, &["install", "/does/not/exist"]);
    assert!(!install.status.success());
}
