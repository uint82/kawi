use std::fs;
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256};

use super::common::*;
use crate::app::App;

const PLUGIN: &str = "wasm-jobs";

fn write_plugin(root: &Path) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kawi-plugin/tests/fixtures/wasm/job-plugin.wasm");
    let bytes = fs::read(fixture).expect("job fixture");
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
"ui.message" = true
"process.spawn" = {{ commands = ["/bin/sh", "/bin/cat", "/bin/sleep"] }}

[[commands]]
id = "start"
title = "Start a job"

[[commands]]
id = "echo"
title = "Start an echo job"

[[commands]]
id = "signal"
title = "Signal a job"

[[commands]]
id = "kill"
title = "Kill a job"

[[commands]]
id = "drain"
title = "Drain job events"
"#,
            Sha256::digest(&bytes)
        ),
    )
    .unwrap();
}

fn wait_for_drain(app: &mut App, job: &str) -> String {
    std::thread::sleep(Duration::from_millis(1100));
    invoke_plugin_args(
        app,
        &format!("{PLUGIN}.drain"),
        Some(toml::Value::String(job.to_string())),
    )
    .unwrap_or_else(|| panic!("drain result: {:?}", app.plugins.logs(PLUGIN)))
}

#[test]
fn wasm_plugin_opens_and_polls_a_job() {
    let root = temp_dir("wasm-job");
    write_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    let job = invoke_plugin_args(&mut app, &format!("{PLUGIN}.start"), None).expect("job id");
    let result = wait_for_drain(&mut app, &job);
    assert!(result.contains("out:hello;"), "{result}");
    assert!(result.contains("err:err;"), "{result}");
    assert!(
        result.contains("exit:JobState::Exited:Some(7):None;"),
        "{result}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_plugin_writes_and_closes_stdin() {
    let root = temp_dir("wasm-job-echo");
    write_plugin(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    let job = invoke_plugin_args(&mut app, &format!("{PLUGIN}.echo"), None).expect("job id");
    let result = wait_for_drain(&mut app, &job);
    assert!(result.contains("out:hello ;"), "{result}");
    assert!(
        result.contains("exit:JobState::Exited:Some(0):None;"),
        "{result}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_plugin_signals_a_job() {
    let root = temp_dir("wasm-job-signal");
    write_plugin(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    let job = invoke_plugin_args(&mut app, &format!("{PLUGIN}.signal"), None).expect("job id");
    let result = wait_for_drain(&mut app, &job);
    assert!(
        result.contains("exit:JobState::Exited:None:Some(15);"),
        "{result}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wasm_plugin_kills_a_job() {
    let root = temp_dir("wasm-job-kill");
    write_plugin(&root);
    let mut app = app();
    load_plugins(&mut app, &root);
    let job = invoke_plugin_args(&mut app, &format!("{PLUGIN}.kill"), None).expect("job id");
    let result = wait_for_drain(&mut app, &job);
    assert!(result.contains("exit:JobState::Killed:None:"), "{result}");
    fs::remove_dir_all(root).unwrap();
}
