use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use kawi_plugin::{VariableScope, VariableTarget};
use sha2::{Digest, Sha256};

use super::common::{app, invoke_plugin, load_plugins, temp_dir};

const MESSAGE_INTERVAL: Duration = Duration::from_millis(1100);

#[test]
fn wasm_plugins_use_timers_variables_and_options() {
    let root = temp_dir("wasm-automation");
    write_plugin(&root);
    let mut app = app();
    let warnings = load_plugins(&mut app, &root);
    assert!(warnings.is_empty(), "{warnings:?}");

    assert_eq!(
        invoke_plugin(&mut app, "wasm.report").as_deref(),
        Some("ready=yes;variables=1;timers=2;last=none")
    );

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, "wasm.fire").as_deref(),
        Some("scheduled"),
        "{:?}",
        app.plugins.logs("wasm")
    );
    wait_for_timer(&mut app);

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(
        invoke_plugin(&mut app, "wasm.report").as_deref(),
        Some("ready=yes;variables=2;timers=2;last=once")
    );

    std::thread::sleep(MESSAGE_INTERVAL);
    assert_eq!(invoke_plugin(&mut app, "wasm.cancel").as_deref(), Some("2"));
    assert!(app.plugins.registered_timers("wasm").is_empty());
    fs::remove_dir_all(root).unwrap();
}

fn wait_for_timer(app: &mut crate::app::App) {
    let target = VariableTarget {
        scope: VariableScope::Plugin,
        buffer: None,
        window: None,
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.plugins.get_variable("wasm", target, "last").is_none() {
        assert!(
            Instant::now() < deadline,
            "timer timed out: {:?} {:?}",
            app.plugins.status("wasm"),
            app.plugins.logs("wasm")
        );
        app.poll_plugins();
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn write_plugin(root: &Path) {
    let bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../kawi-plugin/tests/fixtures/wasm/automation.wasm"),
    )
    .unwrap();
    let dir = root.join("wasm");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        format!(
            r#"name = "wasm"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "plugin.wasm"
sha256 = "sha256:{:x}"

[capabilities]
"editor.schedule" = true
"editor.variables" = true
"options.write" = true
"ui.message" = true

[[commands]]
id = "report"
title = "Report"

[[commands]]
id = "fire"
title = "Fire"

[[commands]]
id = "cancel"
title = "Cancel"
"#,
            Sha256::digest(&bytes)
        ),
    )
    .unwrap();
}
