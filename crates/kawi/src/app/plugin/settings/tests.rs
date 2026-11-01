use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use kawi_plugin::load_catalog;

use super::PluginSettings;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir(name: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "kawi_plugin_settings_{}_{}_{}",
        std::process::id(),
        name,
        id
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn manifest(root: &Path) {
    let dir = root.join("echo");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("plugin.toml"),
        r#"name = "echo"
version = "0.1.0"
protocol = 1
command = ["/bin/true"]

[settings.size]
type = "integer"
default = 10
minimum = 1
maximum = 20

[settings.mode]
type = "string"
default = "auto"
enum = ["auto", "dark"]
"#,
    )
    .unwrap();
}

fn settings_path(root: &Path) -> PathBuf {
    root.join("plugins.toml")
}

#[test]
fn loads_defaults_and_user_values() {
    let root = temp_dir("values");
    manifest(&root);
    fs::write(settings_path(&root), "[echo]\nsize = 4\nmode = \"dark\"\n").unwrap();
    let catalog = load_catalog(&root);
    let mut settings = PluginSettings::default();
    settings.set_path(settings_path(&root));
    let warnings = settings.reload(&catalog);
    assert!(warnings.is_empty(), "{warnings:?}");
    let values = settings.values("echo").expect("values");
    assert_eq!(values["size"], toml::Value::Integer(4));
    assert_eq!(values["mode"], toml::Value::String("dark".to_string()));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_file_uses_defaults() {
    let root = temp_dir("defaults");
    manifest(&root);
    let catalog = load_catalog(&root);
    let mut settings = PluginSettings::default();
    settings.set_path(settings_path(&root));
    assert!(settings.reload(&catalog).is_empty());
    let values = settings.values("echo").expect("values");
    assert_eq!(values["size"], toml::Value::Integer(10));
    assert_eq!(values["mode"], toml::Value::String("auto".to_string()));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_values_and_unknown_keys_warn_and_fall_back() {
    let root = temp_dir("invalid");
    manifest(&root);
    fs::write(
        settings_path(&root),
        "[echo]\nsize = 0\nmode = \"light\"\nunknown = true\n\n[ghost]\nsize = 1\n",
    )
    .unwrap();
    let catalog = load_catalog(&root);
    let mut settings = PluginSettings::default();
    settings.set_path(settings_path(&root));
    let warnings = settings.reload(&catalog);
    assert_eq!(warnings.len(), 4, "{warnings:?}");
    assert!(warnings.iter().any(|warning| warning.contains("size")));
    assert!(warnings.iter().any(|warning| warning.contains("mode")));
    assert!(warnings.iter().any(|warning| warning.contains("unknown")));
    assert!(warnings.iter().any(|warning| warning.contains("ghost")));
    let values = settings.values("echo").expect("values");
    assert_eq!(values["size"], toml::Value::Integer(10));
    assert_eq!(values["mode"], toml::Value::String("auto".to_string()));
    assert!(!values.contains_key("unknown"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reloads_changed_files_and_restores_defaults_when_removed() {
    let root = temp_dir("reload");
    manifest(&root);
    fs::write(settings_path(&root), "[echo]\nsize = 2\n").unwrap();
    let catalog = load_catalog(&root);
    let mut settings = PluginSettings::default();
    settings.set_path(settings_path(&root));
    settings.reload(&catalog);
    assert_eq!(
        settings.values("echo").expect("values")["size"],
        toml::Value::Integer(2)
    );
    fs::write(settings_path(&root), "[echo]\nsize = 3\n").unwrap();
    assert!(settings.ensure(&catalog).is_empty());
    assert_eq!(
        settings.values("echo").expect("values")["size"],
        toml::Value::Integer(3)
    );
    fs::remove_file(settings_path(&root)).unwrap();
    assert!(settings.ensure(&catalog).is_empty());
    assert_eq!(
        settings.values("echo").expect("values")["size"],
        toml::Value::Integer(10)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn parse_errors_fall_back_to_defaults() {
    let root = temp_dir("parse");
    manifest(&root);
    fs::write(settings_path(&root), "[echo\n").unwrap();
    let catalog = load_catalog(&root);
    let mut settings = PluginSettings::default();
    settings.set_path(settings_path(&root));
    let warnings = settings.reload(&catalog);
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        settings.values("echo").expect("values")["size"],
        toml::Value::Integer(10)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn writes_values_and_preserves_other_content() {
    let root = temp_dir("write");
    manifest(&root);
    let catalog = load_catalog(&root);
    fs::write(
        settings_path(&root),
        "# keep this\n[echo]\nsize = 4\nother = \"keep\"\n\n[unrelated]\nvalue = true\n",
    )
    .unwrap();
    let mut settings = PluginSettings::default();
    settings.set_path(settings_path(&root));
    settings.reload(&catalog);
    settings
        .set_value(
            &catalog,
            "echo",
            "mode",
            toml::Value::String("dark".to_string()),
        )
        .unwrap();
    let text = fs::read_to_string(settings_path(&root)).unwrap();
    assert!(text.contains("# keep this"));
    assert!(text.contains("other = \"keep\""));
    assert!(text.contains("[unrelated]"));
    assert_eq!(
        settings.values("echo").expect("values")["mode"],
        toml::Value::String("dark".to_string())
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn writing_the_default_removes_the_override() {
    let root = temp_dir("reset");
    manifest(&root);
    let catalog = load_catalog(&root);
    fs::write(settings_path(&root), "[echo]\nsize = 4\nmode = \"dark\"\n").unwrap();
    let mut settings = PluginSettings::default();
    settings.set_path(settings_path(&root));
    settings.reload(&catalog);
    settings
        .set_value(&catalog, "echo", "size", toml::Value::Integer(10))
        .unwrap();
    let text = fs::read_to_string(settings_path(&root)).unwrap();
    assert!(!text.contains("size"));
    assert!(text.contains("mode"));
    assert_eq!(
        settings.values("echo").expect("values")["size"],
        toml::Value::Integer(10)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_values_without_writing() {
    let root = temp_dir("write_invalid");
    manifest(&root);
    let catalog = load_catalog(&root);
    let path = settings_path(&root);
    let mut settings = PluginSettings::default();
    settings.set_path(path.clone());
    settings.reload(&catalog);
    assert!(settings
        .set_value(&catalog, "echo", "size", toml::Value::Integer(0))
        .is_err());
    assert!(!path.exists());
    assert!(settings
        .set_value(&catalog, "echo", "unknown", toml::Value::Boolean(true))
        .is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_to_rewrite_a_malformed_file() {
    let root = temp_dir("write_malformed");
    manifest(&root);
    let catalog = load_catalog(&root);
    let path = settings_path(&root);
    fs::write(&path, "[echo\n").unwrap();
    let mut settings = PluginSettings::default();
    settings.set_path(path.clone());
    settings.reload(&catalog);
    assert!(settings
        .set_value(&catalog, "echo", "size", toml::Value::Integer(4))
        .is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), "[echo\n");
    fs::remove_dir_all(root).unwrap();
}
