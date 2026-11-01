use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::common::*;

#[test]
fn wasm_plugin_uses_settings_and_composition() {
    let root = temp_dir("wasm-config-composition");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kawi-plugin/tests/fixtures/wasm/config.wasm");
    let bytes = fs::read(fixture).unwrap();
    let dir = root.join("configurable");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        format!(
            r#"name = "configurable"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "plugin.wasm"
sha256 = "sha256:{:x}"
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
"#,
            Sha256::digest(&bytes)
        ),
    )
    .unwrap();
    fs::write(
        root.join("plugins.toml"),
        "[configurable]\nsize = 6\nmode = \"dark\"\n",
    )
    .unwrap();
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        invoke_plugin(&mut app, "configurable.read"),
        Some("6".to_string())
    );
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert_eq!(
        invoke_plugin(&mut app, "configurable.list"),
        Some("mode=dark size=6".to_string())
    );
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert_eq!(
        invoke_plugin(&mut app, "configurable.execute"),
        Some("executed".to_string())
    );
    assert_eq!(app.editor.mode(), kawi_editor::Mode::Insert);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert_eq!(
        invoke_plugin(&mut app, "configurable.commands"),
        Some("mode.insert".to_string())
    );
    fs::remove_dir_all(root).unwrap();
}
