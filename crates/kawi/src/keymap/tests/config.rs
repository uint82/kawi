use std::fs;
use std::path::PathBuf;

use super::super::config::load;
use super::super::{BindingSource, KeyChord, KeyCodeSpec, KeyContext};
use crate::command::CommandRegistry;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kawi_keymap_config_{}_{}_{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test"),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn load_text(name: &str, text: &str) -> super::super::config::KeymapLoad {
    let dir = temp_dir(name);
    let path = dir.join("keymaps.toml");
    fs::write(&path, text).unwrap();
    let loaded = load(&path, &CommandRegistry::new());
    let _ = fs::remove_dir_all(dir);
    loaded
}

#[test]
fn missing_file_uses_defaults() {
    let dir = temp_dir("missing");
    let loaded = load(&dir.join("keymaps.toml"), &CommandRegistry::new());
    assert!(loaded.bindings.is_empty());
    assert_eq!(loaded.leader.len(), 2);
    assert!(loaded.warnings.is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn loads_modes_leader_and_timeout() {
    let loaded = load_text(
        "modes",
        r#"
leader = ","
timeout = 200

[normal]
"<leader>ff" = "picker.files"
"gx" = { command = "file.save", desc = "save", when = "editor" }

[insert]
"jk" = "mode.normal"

[visual]
"Q" = { command = "keys.send", args = { keys = "gq" } }
"#,
    );
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.timeout.as_millis(), 200);
    assert_eq!(loaded.leader.len(), 1);
    assert_eq!(loaded.leader[0].code, KeyCodeSpec::Char(','));
    assert_eq!(loaded.bindings.len(), 4);
    let normal = loaded
        .bindings
        .iter()
        .find(|binding| binding.mode == KeyContext::Normal && binding.keys.to_string() == ",ff")
        .expect("leader binding");
    assert_eq!(normal.source, BindingSource::User);
    let described = loaded
        .bindings
        .iter()
        .find(|binding| binding.keys.to_string() == "gx")
        .expect("described");
    assert_eq!(described.description.as_deref(), Some("save"));
    assert_eq!(described.when.as_deref(), Some("editor"));
    let insert = loaded
        .bindings
        .iter()
        .find(|binding| binding.mode == KeyContext::Insert)
        .expect("insert");
    assert_eq!(insert.keys.to_string(), "jk");
    let send = loaded
        .bindings
        .iter()
        .find(|binding| binding.mode == KeyContext::Visual)
        .expect("send");
    assert!(send.args.is_some());
}

#[test]
fn warns_about_invalid_entries() {
    let loaded = load_text(
        "warnings",
        r#"
unknown = true

[normal]
"<C-w" = "file.save"
"2x" = "file.save"
"gx" = "plugin.missing"
"gy" = { command = "file.save", args = [1] }
"bad" = 42
"#,
    );
    assert_eq!(loaded.warnings.len(), 5, "{:?}", loaded.warnings);
    assert_eq!(loaded.bindings.len(), 1);
    let lines: Vec<String> = loaded
        .warnings
        .iter()
        .map(|warning| warning.format(std::path::Path::new("keymaps.toml")))
        .collect();
    assert!(lines.iter().any(|line| line.contains("unknown")));
    assert!(lines.iter().any(|line| line.contains("unterminated")));
    assert!(lines.iter().any(|line| line.contains("reserved")));
    assert!(lines.iter().any(|line| line.contains("unknown command")));
    assert!(lines.iter().any(|line| line.contains("expected a command")));
}

#[test]
fn warns_about_invalid_document() {
    let loaded = load_text("invalid", "[normal\n");
    assert_eq!(loaded.warnings.len(), 1);
    assert!(loaded.bindings.is_empty());
}

#[test]
fn leader_values_must_be_single_chords() {
    let loaded = load_text("leader", "leader = \"ab\"\n");
    assert_eq!(loaded.leader.len(), 2);
    assert_eq!(loaded.warnings.len(), 1);
}

#[test]
fn leaders_are_deduplicated() {
    let loaded = load_text("dedup", "leader = [\" \", \"<Space>\"]\n");
    assert_eq!(loaded.leader.len(), 1);
    assert_eq!(loaded.warnings.len(), 0);
}

#[test]
fn default_leader_has_space_and_backslash() {
    let leaders: Vec<KeyChord> = super::super::config::KeymapLoad::default().leader;
    assert_eq!(leaders[0].code, KeyCodeSpec::Char(' '));
    assert_eq!(leaders[1].code, KeyCodeSpec::Char('\\'));
}

#[test]
fn warns_about_unknown_binding_fields() {
    let loaded = load_text(
        "unknown_field",
        "[normal]\n\"gx\" = { command = \"file.save\", unknown = true }\n",
    );
    assert_eq!(loaded.warnings.len(), 1);
    assert!(loaded.bindings.is_empty());
}

#[test]
fn accepts_inline_mode_tables() {
    let loaded = load_text("inline_mode", "normal = { \"gx\" = \"file.save\" }\n");
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.bindings.len(), 1);
}
