#![cfg(feature = "wasm-grammars")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("kawi_grammar_cli_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&root);
        for directory in ["data", "config", "state", "cache"] {
            fs::create_dir_all(root.join(directory)).expect("workspace directory");
        }
        Self { root }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kawi"));
        command
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("XDG_CACHE_HOME", self.root.join("cache"));
        command
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
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

#[test]
fn installs_and_reports_a_pack() {
    let workspace = Workspace::new("install");
    let empty = grammar(&workspace, &["status"]);
    assert!(empty.status.success(), "{:?}", empty.stderr);
    assert!(stdout(empty).contains("no grammar packs installed"));

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../kawi-grammar/fixtures/adl/0.7.0");
    let install = grammar(
        &workspace,
        &["install", fixture.to_str().expect("fixture path")],
    );
    assert!(install.status.success(), "{:?}", install.stderr);
    assert!(stdout(install).contains("installed adl 0.7.0"));

    let status = grammar(&workspace, &["status"]);
    assert!(status.status.success(), "{:?}", status.stderr);
    assert!(stdout(status).contains("adl 0.7.0 [user] valid"));
}

#[test]
fn rejects_a_missing_pack() {
    let workspace = Workspace::new("missing");
    let install = grammar(&workspace, &["install", "/does/not/exist"]);
    assert!(!install.status.success());
}
