use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Workspace {
    root: PathBuf,
    config: PathBuf,
    state: PathBuf,
    project: PathBuf,
}

impl Workspace {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("kawi_doctor_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&root);
        let config = root.join("config");
        let state = root.join("state");
        let project = root.join("project");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(&state).unwrap();
        fs::create_dir_all(&project).unwrap();
        Self {
            root,
            config,
            state,
            project,
        }
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn doctor(workspace: &Workspace, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kawi"))
        .arg("doctor")
        .args(args)
        .env("XDG_CONFIG_HOME", &workspace.config)
        .env("XDG_STATE_HOME", &workspace.state)
        .output()
        .expect("run kawi doctor")
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("doctor JSON")
}

fn check_ids(report: &Value) -> Vec<&str> {
    report["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .map(|check| check["id"].as_str().expect("check id"))
        .collect()
}

#[test]
fn doctor_reports_the_workspace_without_creating_state() {
    let workspace = Workspace::new("healthy");
    let output = doctor(
        &workspace,
        &[
            "--json",
            "--path",
            workspace.project.to_str().expect("project path"),
        ],
    );
    assert!(output.status.success(), "{output:?}");
    let report = report(&output);
    assert_eq!(report["schema"], 1);
    assert_eq!(report["kawi_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        report["workspace"],
        fs::canonicalize(&workspace.project)
            .unwrap()
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(
        check_ids(&report),
        [
            "paths",
            "config",
            "keymaps",
            "plugin-keybindings",
            "plugins",
            "languages",
            "tasks",
            "git",
            "session",
            "state"
        ]
    );
    assert!(!workspace.state.join("kawi").exists());
}

#[test]
fn strict_mode_fails_for_configuration_warnings() {
    let workspace = Workspace::new("invalid_config");
    let config = workspace.config.join("kawi");
    fs::create_dir_all(&config).unwrap();
    fs::write(config.join("config.toml"), "[ui\n").unwrap();
    let output = doctor(
        &workspace,
        &[
            "--json",
            "--strict",
            "--path",
            workspace.project.to_str().expect("project path"),
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let report = report(&output);
    assert!(report["summary"]["warn"].as_u64().unwrap() >= 1);
}

#[test]
fn doctor_fails_when_state_cannot_be_written() {
    let workspace = Workspace::new("bad_state");
    let state = workspace.root.join("state-file");
    fs::write(&state, "not a directory").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_kawi"))
        .arg("doctor")
        .args([
            "--json",
            "--path",
            workspace.project.to_str().expect("project path"),
        ])
        .env("XDG_CONFIG_HOME", &workspace.config)
        .env("XDG_STATE_HOME", &state)
        .output()
        .expect("run kawi doctor");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let report = report(&output);
    assert!(report["summary"]["error"].as_u64().unwrap() >= 1);
    assert_eq!(report["summary"]["status"], "error");
}

#[test]
fn invalid_options_exit_with_usage_status() {
    let workspace = Workspace::new("usage");
    let output = doctor(&workspace, &["--unknown"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("Usage: kawi doctor"));
}

#[test]
fn general_help_and_version_are_available() {
    let version = Command::new(env!("CARGO_BIN_EXE_kawi"))
        .arg("--version")
        .output()
        .expect("run kawi version");
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim(),
        format!("kawi {}", env!("CARGO_PKG_VERSION"))
    );
    let help = Command::new(env!("CARGO_BIN_EXE_kawi"))
        .arg("doctor")
        .arg("--help")
        .output()
        .expect("run kawi doctor help");
    assert!(help.status.success());
    assert!(String::from_utf8(help.stdout)
        .unwrap()
        .contains("Usage: kawi doctor"));
}

#[test]
fn doctor_accepts_a_file_target() {
    let workspace = Workspace::new("file_target");
    let file = workspace.project.join("main.rs");
    fs::write(&file, "fn main() {}\n").unwrap();
    let output = doctor(
        &workspace,
        &["--json", "--path", file.to_str().expect("file path")],
    );
    assert!(output.status.success(), "{output:?}");
    let report = report(&output);
    assert_eq!(
        report["workspace"],
        canonical(&workspace.project).to_string_lossy().as_ref()
    );
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap()
}
