use std::fs;
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256};

use super::common::{app, invoke_plugin, load_plugins, temp_dir};

const PLUGIN: &str = "wasm-panels";
const MESSAGE_INTERVAL: Duration = Duration::from_millis(1100);

fn write_plugin(root: &Path) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kawi-plugin/tests/fixtures/wasm/panel-plugin.wasm");
    let bytes = fs::read(fixture).expect("panel fixture");
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
            Sha256::digest(&bytes)
        ),
    )
    .unwrap();
}

#[test]
fn wasm_plugin_controls_a_panel() {
    let root = temp_dir("wasm-panels");
    write_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");

    let opened = invoke_plugin(&mut app, &format!("{PLUGIN}.open")).expect("open message");
    assert!(opened.starts_with("opened:"), "{opened}");
    let view = app.plugin_panel_view().expect("panel view");
    assert_eq!(view.title, "Index");
    assert_eq!(view.status, "building");
    assert_eq!(view.lines.len(), 2);
    assert!(!view.focused);

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, &format!("{PLUGIN}.update")),
        Some("updated".to_string()),
        "{:?}",
        app.plugins.logs(PLUGIN)
    );
    let view = app.plugin_panel_view().expect("panel view");
    assert_eq!(view.status, "complete");
    assert_eq!(view.lines[0].text, "gamma");
    assert_eq!(app.plugin_panel_rect().unwrap().height, 5);

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, &format!("{PLUGIN}.focus")),
        Some("focused".to_string())
    );
    assert!(app.plugin_panel_view().expect("panel view").focused);

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, &format!("{PLUGIN}.close")),
        Some("closed".to_string())
    );
    assert!(app.plugin_panel_view().is_none());
    fs::remove_dir_all(root).unwrap();
}
