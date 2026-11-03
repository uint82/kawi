use std::fs;
use std::time::Duration;

use super::common::{app, invoke_plugin, load_plugins, temp_dir};

const PLUGIN: &str = "panels";
const MESSAGE_INTERVAL: Duration = Duration::from_millis(1100);

fn write_plugin(root: &std::path::Path) {
    let dir = root.join(PLUGIN);
    fs::create_dir_all(&dir).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/plugin/test_data/panel_plugin.py");
    fs::copy(source, dir.join("panel_plugin.py")).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "panels"
version = "0.1.0"
protocol = 1
command = ["python3", "${plugin_dir}/panel_plugin.py"]
timeout_ms = 5000

[capabilities]
"ui.message" = true
"ui.panel" = true

[[commands]]
id = "open"
title = "Open panel"

[[commands]]
id = "update"
title = "Update panel"

[[commands]]
id = "focus"
title = "Focus panel"

[[commands]]
id = "close"
title = "Close panel"
"#,
    )
    .unwrap();
}

#[test]
fn process_plugin_controls_a_panel() {
    let root = temp_dir("panels");
    write_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");

    let opened = invoke_plugin(&mut app, "panels.open").expect("open message");
    assert!(opened.starts_with("opened:"), "{opened}");
    let view = app.plugin_panel_view().expect("panel view");
    assert_eq!(view.title, "Index");
    assert_eq!(view.status, "building");
    assert_eq!(view.lines.len(), 2);
    assert!(!view.focused);

    std::thread::sleep(MESSAGE_INTERVAL);
    let updated = invoke_plugin(&mut app, "panels.update");
    assert_eq!(
        updated,
        Some("updated".to_string()),
        "{:?}",
        app.plugins.logs(PLUGIN)
    );
    let view = app.plugin_panel_view().expect("panel view");
    assert_eq!(view.status, "complete");
    assert_eq!(view.lines[0].text, "gamma");

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, "panels.focus"),
        Some("focused".to_string())
    );
    assert!(app.plugin_panel_view().expect("panel view").focused);

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, "panels.close"),
        Some("closed".to_string())
    );
    assert!(app.plugin_panel_view().is_none());
    fs::remove_dir_all(root).unwrap();
}
