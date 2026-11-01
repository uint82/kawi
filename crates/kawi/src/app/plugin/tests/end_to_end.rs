use std::fs;
use std::time::Duration;

use kawi_editor::Mode;

use super::common::{app, invoke_plugin, load_plugins, temp_dir};

const PLUGIN: &str = "configurable";
const MESSAGE_INTERVAL: Duration = Duration::from_millis(1100);
const SCRIPT: &str = "config_plugin.py";

fn write_plugin(root: &std::path::Path) {
    let dir = root.join(PLUGIN);
    fs::create_dir_all(&dir).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/plugin/test_data")
        .join(SCRIPT);
    fs::copy(source, dir.join(SCRIPT)).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        format!(
            r#"name = "{PLUGIN}"
version = "0.1.0"
protocol = 1
command = ["python3", "${{plugin_dir}}/{SCRIPT}"]
timeout_ms = 5000

[capabilities]
"ui.message" = true
"editor.execute" = {{ commands = ["mode.insert"] }}

[settings.size]
type = "integer"
default = 10
minimum = 1
maximum = 20

[settings.mode]
type = "string"
default = "auto"
enum = ["auto", "dark"]

[[commands]]
id = "read"
title = "Read a setting"

[[commands]]
id = "list"
title = "List settings"

[[commands]]
id = "execute"
title = "Execute a command"

[[commands]]
id = "commands"
title = "List composable commands"
"#
        ),
    )
    .unwrap();
}

#[test]
fn process_plugin_reads_and_reloads_settings() {
    let root = temp_dir("config-process-read");
    write_plugin(&root);
    fs::write(root.join("plugins.toml"), "[configurable]\nsize = 4\n").unwrap();
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        invoke_plugin(&mut app, "configurable.read"),
        Some("4".to_string())
    );
    fs::write(root.join("plugins.toml"), "[configurable]\nsize = 8\n").unwrap();
    std::thread::sleep(MESSAGE_INTERVAL);
    let message = invoke_plugin(&mut app, "configurable.read");
    assert_eq!(
        message,
        Some("8".to_string()),
        "{:?}",
        app.plugins.logs(PLUGIN)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn process_plugin_lists_settings_and_runs_composition() {
    let root = temp_dir("config-process-compose");
    write_plugin(&root);
    fs::write(
        root.join("plugins.toml"),
        "[configurable]\nsize = 4\nmode = \"dark\"\n",
    )
    .unwrap();
    let mut app = app();
    load_plugins(&mut app, &root);
    assert_eq!(
        invoke_plugin(&mut app, "configurable.list"),
        Some("mode=dark size=4".to_string())
    );
    std::thread::sleep(MESSAGE_INTERVAL);
    let message = invoke_plugin(&mut app, "configurable.execute");
    assert_eq!(
        message,
        Some("executed".to_string()),
        "{:?}",
        app.plugins.logs(PLUGIN)
    );
    assert_eq!(app.editor.mode(), Mode::Insert);
    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, "configurable.commands"),
        Some("mode.insert".to_string())
    );
    fs::remove_dir_all(root).unwrap();
}
