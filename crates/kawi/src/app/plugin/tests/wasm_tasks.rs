use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::common::*;
use crate::app::App;

const PLUGIN: &str = "wasm-tasks";

#[test]
fn wasm_plugin_lists_only_scoped_configured_tasks() {
    let (mut app, root) = setup();
    let result = invoke_plugin(&mut app, &format!("{PLUGIN}.list")).expect("task list");
    assert_eq!(result, "fail,pass,sleep");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_plugin_runs_a_task_and_polls_its_events() {
    let (mut app, root) = setup();
    let result = run(&mut app, "pass");
    assert!(result.starts_with("0|E|0|T|F|0|0|"), "{result}");
    assert!(result.contains("|out=out"), "{result}");
    assert!(result.contains("|err=err"), "{result}");
    assert!(result.contains("|end=E0T"), "{result}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_plugin_reports_findings_and_a_failing_exit_code() {
    let (mut app, root) = setup();
    let result = run(&mut app, "fail");
    assert!(result.starts_with("0|E|3|F|F|1|0|"), "{result}");
    assert!(result.contains("|find=2:3:E:bad"), "{result}");
    assert!(result.contains("|end=E3F"), "{result}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_plugin_starts_and_drains_a_task_across_invocations() {
    let (mut app, root) = setup();
    let job = invoke_plugin_args(&mut app, &format!("{PLUGIN}.start"), text("pass"))
        .expect("started job");
    assert_eq!(job, "0");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let result = invoke_plugin_args(&mut app, &format!("{PLUGIN}.drain"), text(&job))
        .unwrap_or_else(|| {
            panic!(
                "drained task: {:?} {:?}",
                app.plugins.status(PLUGIN),
                app.plugins.logs(PLUGIN)
            )
        });
    assert!(result.starts_with("E|0|T|F|0|0|"), "{result}");
    assert!(result.contains("|end=E0T"), "{result}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_plugin_cancels_a_running_task() {
    let (mut app, root) = setup();
    let result = invoke_plugin_args(&mut app, &format!("{PLUGIN}.cancel"), text("sleep"))
        .expect("cancelled task");
    assert!(result.starts_with("C|-|F|T|0|0|"), "{result}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_plugin_rejects_a_missing_task() {
    let (mut app, root) = setup();
    assert_eq!(
        run(&mut app, "missing"),
        "err=-32062:task not found: missing"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_plugin_rejects_an_out_of_scope_task() {
    let (mut app, root) = setup();
    assert_eq!(
        run(&mut app, "secret"),
        "err=-32012:capability denied: task.run"
    );
    fs::remove_dir_all(root).unwrap();
}

fn run(app: &mut App, task: &str) -> String {
    invoke_plugin_args(app, &format!("{PLUGIN}.run"), text(task))
        .unwrap_or_else(|| panic!("run result: {:?}", app.plugins.logs(PLUGIN)))
}

fn text(value: &str) -> Option<toml::Value> {
    Some(toml::Value::String(value.to_string()))
}

fn setup() -> (App, std::path::PathBuf) {
    let root = temp_dir("wasm-tasks");
    write_plugin(&root);
    let mut app = app();
    let loaded = kawi_task::load(&root.join("tasks.toml"));
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    app.set_tasks(loaded.tasks);
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    (app, root)
}

fn write_plugin(root: &Path) {
    let bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../kawi-plugin/tests/fixtures/wasm/tasks-plugin.wasm"),
    )
    .expect("task fixture");
    let dir = root.join(PLUGIN);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        format!(
            r#"name = "{PLUGIN}"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "plugin.wasm"
sha256 = "sha256:{:x}"
timeout_ms = 5000

[capabilities]
"task.run" = {{ tasks = ["fail", "missing", "pass", "sleep"] }}
"ui.message" = true

[[commands]]
id = "list"
title = "List tasks"

[[commands]]
id = "run"
title = "Run a task"

[[commands]]
id = "start"
title = "Start a task"

[[commands]]
id = "drain"
title = "Drain a task"

[[commands]]
id = "cancel"
title = "Cancel a task"
"#,
            Sha256::digest(&bytes)
        ),
    )
    .unwrap();
    fs::write(
        root.join("tasks.toml"),
        r#"[task.pass]
command = "sh"
args = ["-c", "printf out; printf err >&2"]

[task.fail]
command = "sh"
args = ["-c", "printf 'src/a.go:2:3: bad'; exit 3"]
parser = "go"

[task.sleep]
command = "sh"
args = ["-c", "sleep 10"]
"#,
    )
    .unwrap();
}
