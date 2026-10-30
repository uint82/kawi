use std::fs;
use std::time::Duration;

use super::*;

#[test]
fn command_gate_denies_at_enqueue() {
    let root = temp_dir("queue_gate_enqueue");
    queue_plugin(&root);
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.queue.grant]\n\"editor.read\" = true\n\"ui.message\" = true\n",
    );
    let target = target(&app, "queue.report");
    app.run_plugin_command(target, args(0, true)).unwrap();
    assert_eq!(app.plugins.queue_depth("queue"), 0);
    assert!(app
        .message
        .as_deref()
        .is_some_and(|message| message.contains("capability denied: editor.write")));
    assert!(app
        .plugins
        .logs("queue")
        .iter()
        .any(|line| line.contains("denied editor.write")));
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ungated_command_runs_without_command_capability() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = temp_dir("queue_gate_ungated");
    queue_plugin(&root);
    let mut app = app();
    load_plugins_with_permissions(
        &mut app,
        &root,
        "[plugins.queue.grant]\n\"editor.read\" = true\n\"ui.message\" = true\n",
    );
    let target = target(&app, "queue.plain");
    app.run_plugin_command(target, args(0, true)).unwrap();
    poll_until(&mut app, Duration::from_secs(5), |app| {
        app.message
            .as_deref()
            .is_some_and(|message| message.starts_with("revision:"))
    });
    assert_eq!(app.plugins.queue_depth("queue"), 0);
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}
