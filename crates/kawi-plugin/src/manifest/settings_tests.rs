use super::test_support::{temp_dir, valid_manifest, write_manifest};
use super::*;
use std::fs;

#[test]
fn parses_and_normalizes_settings() {
    let root = temp_dir("settings");
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
    let root = temp_dir("settings");
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
