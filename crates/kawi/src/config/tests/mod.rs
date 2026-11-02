use super::*;
use crate::keymap::Flavor;
use kawi_editor::{DEFAULT_HISTORY_LIMIT, MAX_HISTORY_LIMIT};
use std::path::PathBuf;

mod statusline;
mod themes;

fn temp_dir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let dir = std::env::temp_dir().join(format!(
        "kawi_config_{}_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        name
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn load_text(text: &str) -> ConfigLoad {
    let dir = temp_dir("load");
    let path = dir.join("config.toml");
    fs::write(&path, text).unwrap();
    let loaded = load(&path);
    let _ = fs::remove_dir_all(dir);
    loaded
}

#[test]
fn missing_file_uses_defaults_without_warnings() {
    let dir = temp_dir("missing");
    let loaded = load(&dir.join("config.toml"));
    assert_eq!(loaded.config, Config::default());
    assert!(loaded.warnings.is_empty());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn loads_ui_editor_and_history_sections() {
    let loaded = load_text(
        r#"
[ui]
keymap = "helix"
theme = "gruvbox-dark"

[editor]
shiftwidth = 2
expandtab = true
format_on_save = true
number = false
ignorecase = true
autoindent = false
auto_pairs = false
autosave = "focus"
autosave_delay = 250
foldmethod = "indent"
foldcolumn = 0

[history]
limit = 25
undo_persist = false
undo_exclude = ["/tmp/*", "*.gpg", "*/.ssh/*"]
"#,
    );
    assert!(loaded.warnings.is_empty());
    assert_eq!(
        loaded.config.get("keymap"),
        Some(&OptionValue::Text("helix".to_string()))
    );
    assert_eq!(
        loaded.config.get("theme"),
        Some(&OptionValue::Text("gruvbox-dark".to_string()))
    );
    assert_eq!(loaded.config.get("shiftwidth"), Some(&OptionValue::Int(2)));
    assert_eq!(
        loaded.config.get("foldmethod"),
        Some(&OptionValue::Text("indent".to_string()))
    );
    assert_eq!(loaded.config.get("foldcolumn"), Some(&OptionValue::Int(0)));
    assert_eq!(
        loaded.config.get("expandtab"),
        Some(&OptionValue::Bool(true))
    );
    assert_eq!(loaded.config.get("number"), Some(&OptionValue::Bool(false)));
    assert_eq!(
        loaded.config.get("ignorecase"),
        Some(&OptionValue::Bool(true))
    );
    assert_eq!(
        loaded.config.get("autoindent"),
        Some(&OptionValue::Bool(false))
    );
    assert_eq!(
        loaded.config.get("auto_pairs"),
        Some(&OptionValue::Bool(false))
    );
    assert_eq!(
        loaded.config.get("autosave"),
        Some(&OptionValue::Text("focus".to_string()))
    );
    assert_eq!(
        loaded.config.get("autosave_delay"),
        Some(&OptionValue::Int(250))
    );
    assert_eq!(loaded.config.get("history"), Some(&OptionValue::Int(25)));
    assert_eq!(
        loaded.config.get("undo_persist"),
        Some(&OptionValue::Bool(false))
    );
    assert_eq!(
        loaded.config.undo_exclude(),
        Some(
            ["/tmp/*", "*.gpg", "*/.ssh/*"]
                .map(str::to_string)
                .as_slice()
        )
    );
}

#[test]
fn rejects_invalid_undo_exclusions() {
    let loaded = load_text("[history]\nundo_exclude = \"/tmp/*\"\n");
    assert!(loaded.config.undo_exclude().is_none());
    assert_eq!(loaded.warnings.len(), 1);
}

#[test]
fn accepts_aliases_in_editor_section() {
    let loaded = load_text("[editor]\nsw = 2\net = true\nfdm = \"indent\"\nfdc = 0\n");
    assert!(loaded.warnings.is_empty());
    assert_eq!(loaded.config.get("shiftwidth"), Some(&OptionValue::Int(2)));
    assert_eq!(
        loaded.config.get("foldmethod"),
        Some(&OptionValue::Text("indent".to_string()))
    );
    assert_eq!(loaded.config.get("foldcolumn"), Some(&OptionValue::Int(0)));
    assert_eq!(
        loaded.config.get("expandtab"),
        Some(&OptionValue::Bool(true))
    );
}

#[test]
fn loads_legacy_top_level_history() {
    let loaded = load_text("history = 25\n");
    assert_eq!(loaded.config.get("history"), Some(&OptionValue::Int(25)));
    assert!(loaded.warnings.is_empty());
}

#[test]
fn rejects_invalid_values_with_their_default() {
    let loaded = load_text(
        r#"
[editor]
numberwidth = 0
expandtab = "yes"
shiftwidth = 4
"#,
    );
    assert_eq!(loaded.config.get("numberwidth"), Some(&OptionValue::Int(4)));
    assert_eq!(
        loaded.config.get("expandtab"),
        Some(&OptionValue::Bool(false))
    );
    assert_eq!(loaded.config.get("shiftwidth"), Some(&OptionValue::Int(4)));
    assert_eq!(loaded.warnings.len(), 2);
    let lines: Vec<String> = loaded
        .warnings
        .iter()
        .map(|warning| warning.format(Path::new("/tmp/config.toml")))
        .collect();
    assert_eq!(
        lines[0],
        r#"/tmp/config.toml:3: editor.numberwidth: rejected 0: using 4"#
    );
    assert_eq!(
        lines[1],
        r#"/tmp/config.toml:4: editor.expandtab: rejected "yes": using false"#
    );
}

#[test]
fn warns_about_unknown_keys_and_sections() {
    let loaded = load_text(
        r#"
[editor]
wat = 1

[ui]
keymap = "vim"
tone = "dark"

[other]
x = 1
"#,
    );
    assert_eq!(
        loaded.config.get("keymap"),
        Some(&OptionValue::Text("vim".to_string()))
    );
    assert_eq!(loaded.warnings.len(), 3);
    assert!(loaded.warnings[0]
        .format(Path::new("config.toml"))
        .contains("editor.wat"));
    assert!(loaded.warnings[1]
        .format(Path::new("config.toml"))
        .contains("ui.tone"));
    assert!(loaded.warnings[2]
        .format(Path::new("config.toml"))
        .contains("other"));
}

#[test]
fn syntax_errors_use_all_defaults() {
    let loaded = load_text("[editor\nshiftwidth = 2\n");
    assert_eq!(loaded.config, Config::default());
    assert_eq!(loaded.warnings.len(), 1);
    let line = loaded.warnings[0].format(Path::new("config.toml"));
    assert!(line.starts_with("config.toml:1:"));
    assert!(line.contains("built-in defaults"));
}

#[test]
fn warning_log_is_rewritten_and_removed() {
    let dir = temp_dir("log");
    let config = dir.join("config.toml");
    let log = dir.join("config.log");
    let warnings =
        vec![ConfigWarning::new(Some(2), "editor.number", "false", "true").format(&config)];
    write_warning_log(&log, &warnings).unwrap();
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        format!(
            "{}:2: editor.number: rejected false: using true\n",
            config.display()
        )
    );
    let replacement =
        vec![ConfigWarning::new(Some(4), "ui.theme", "nope", "default").format(&config)];
    write_warning_log(&log, &replacement).unwrap();
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        format!(
            "{}:4: ui.theme: rejected nope: using default\n",
            config.display()
        )
    );
    write_warning_log(&log, &[]).unwrap();
    assert!(!log.exists());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn history_range_matches_the_editor_limit() {
    let loaded = load_text(&format!("[history]\nlimit = {}\n", MAX_HISTORY_LIMIT + 1));
    assert_eq!(
        loaded.config.get("history"),
        Some(&OptionValue::Int(DEFAULT_HISTORY_LIMIT))
    );
    assert_eq!(loaded.warnings.len(), 1);
}

#[test]
fn keymap_values_use_the_runtime_validator() {
    let loaded = load_text("[ui]\nkeymap = \"personal\"\n");
    assert_eq!(
        loaded.config.get("keymap"),
        Some(&OptionValue::Text("vim".to_string()))
    );
    assert_eq!(loaded.warnings.len(), 1);
    assert_eq!(crate::keymap::flavor_from_name("vim"), Some(Flavor::Vim));
}

#[test]
fn directory_sort_options_accept_keys_and_flags() {
    let loaded = load_text(
        "[explorer]\nsort = \"size\"\nsort_reverse = true\nsort_directories_first = false\n",
    );
    assert!(loaded.warnings.is_empty());
    assert_eq!(
        loaded.config.get("directory_sort"),
        Some(&OptionValue::Text("size".to_string()))
    );
    assert_eq!(
        loaded.config.get("directory_sort_reverse"),
        Some(&OptionValue::Bool(true))
    );
    assert_eq!(
        loaded.config.get("directory_sort_directories_first"),
        Some(&OptionValue::Bool(false))
    );
}

#[test]
fn directory_metadata_accepts_lists_and_rejects_unknown_columns() {
    let loaded = load_text("[explorer]\nmetadata = [\"size\", \"permissions\"]\n");
    assert!(loaded.warnings.is_empty());
    assert_eq!(
        loaded.config.get("directory_metadata"),
        Some(&OptionValue::Text("size,permissions".to_string()))
    );
    let loaded = load_text("[explorer]\nmetadata = []\n");
    assert!(loaded.warnings.is_empty());
    assert_eq!(
        loaded.config.get("directory_metadata"),
        Some(&OptionValue::Text("none".to_string()))
    );
    let loaded = load_text("[explorer]\nmetadata = [\"mode\"]\n");
    assert_eq!(loaded.warnings.len(), 1);
    assert_eq!(
        loaded.config.get("directory_metadata"),
        Some(&OptionValue::Text(
            "permissions,links,owner,group,size,modified".to_string()
        ))
    );
}
