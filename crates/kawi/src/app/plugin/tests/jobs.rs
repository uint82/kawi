use std::fs;
use std::time::{Duration, Instant};

use super::common::{app, invoke_plugin, load_plugins, temp_dir};

const PLUGIN: &str = "jobs";

fn write_plugin(root: &std::path::Path) {
    let dir = root.join(PLUGIN);
    fs::create_dir_all(&dir).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/plugin/test_data/job_plugin.py");
    fs::copy(source, dir.join("job_plugin.py")).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "jobs"
version = "0.1.0"
protocol = 1
command = ["python3", "${plugin_dir}/job_plugin.py"]
timeout_ms = 5000

[capabilities]
"ui.message" = true
"process.spawn" = { commands = ["/bin/sh", "/bin/sleep"] }

[[commands]]
id = "run"
title = "Run a job"

[[commands]]
id = "signal"
title = "Signal a job"
"#,
    )
    .unwrap();
}

#[test]
fn process_plugin_receives_job_events_while_idle() {
    let root = temp_dir("jobs");
    write_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        invoke_plugin(&mut app, "jobs.run"),
        Some("hello|exited|0".to_string()),
        "{:?}",
        app.plugins.logs(PLUGIN)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn process_plugin_signals_a_job() {
    let root = temp_dir("jobs-signal");
    write_plugin(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    assert_eq!(
        invoke_plugin(&mut app, "jobs.signal"),
        Some("|exited|None|15".to_string()),
        "{:?}",
        app.plugins.logs(PLUGIN)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stopping_a_plugin_kills_its_jobs() {
    let root = temp_dir("jobs-stop");
    write_plugin(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    assert_eq!(
        invoke_plugin(&mut app, "jobs.run"),
        Some("hello|exited|0".to_string())
    );
    app.plugins.stop_jobs(PLUGIN);
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.plugins.status(PLUGIN) == Some(kawi_plugin::PluginStatus::Ready)
        && Instant::now() < deadline
    {
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
    fs::remove_dir_all(root).unwrap();
}
