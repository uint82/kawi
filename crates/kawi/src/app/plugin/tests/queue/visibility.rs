use super::*;
use std::fs;

#[test]
fn list_shows_busy_depth_and_logs_counters() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let (root, mut app, target) = setup("queue_visible");
    app.run_plugin_command(target, args(1200, false)).unwrap();
    app.run_plugin_command(target, args(0, true)).unwrap();
    app.handle_plugin_command("list");
    let lines = app.pager.as_ref().expect("pager");
    assert!(lines.iter().any(|line| line.contains("busy (1 queued)")));
    assert!(app
        .plugins
        .logs("queue")
        .iter()
        .any(|line| line == "queue: 1 queued, 0 rejected, 0 dropped"));
    app.plugins.shutdown();
    fs::remove_dir_all(root).unwrap();
}
