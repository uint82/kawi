use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("kawi-plugin-settings-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_manifest(root: &Path, name: &str, body: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(MANIFEST_FILE), body).unwrap();
}

fn valid_manifest(name: &str) -> String {
    format!(
        r#"name = "{name}"
version = "0.1.0"
protocol = 1
command = ["python3", "${{plugin_dir}}/plugin.py"]

[[commands]]
id = "ping"
title = "Ping"
"#
    )
}

#[test]
fn parses_and_normalizes_settings() {
    let root = temp_dir();
    let body = valid_manifest("echo").replacen(
        "\n[[commands]]",
        "\n[settings.size]\ntype = \"integer\"\ndefault = 10\nminimum = 1\nmaximum = 20\n\n[settings.mode]\ntype = \"string\"\ndefault = \"auto\"\nenum = [\"auto\", \"dark\"]\n\n[[commands]]",
        1,
    );
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
    let settings = &catalog.plugins[0].manifest.settings;
    assert_eq!(settings["size"].kind, PluginSettingKind::Integer);
    assert_eq!(settings["size"].default, toml::Value::Integer(10));
    assert_eq!(settings["mode"].choices, ["auto", "dark"]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_settings() {
    let root = temp_dir();
    let body = valid_manifest("echo").replacen(
        "\n[[commands]]",
        "\n[settings.size]\ntype = \"integer\"\ndefault = 0\nminimum = 1\n\n[[commands]]",
        1,
    );
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert!(catalog.plugins.is_empty());
    assert!(catalog.warnings[0].message.contains("setting size"));
    fs::remove_dir_all(root).unwrap();
}
