use anyhow::Result;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use toml_edit::{Document, Item, Value};

use crate::options::{self, ConfigSection, OptionInput, OptionSpec, OptionValue};
use crate::persist::write_private_atomically;
use crate::text::one_line;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Config {
    values: BTreeMap<&'static str, OptionValue>,
}

impl Config {
    pub(crate) fn values(&self) -> &BTreeMap<&'static str, OptionValue> {
        &self.values
    }

    pub(crate) fn get(&self, name: &str) -> Option<&OptionValue> {
        options::canonical_name(name).and_then(|name| self.values.get(name))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConfigWarning {
    line: Option<usize>,
    key: String,
    value: String,
    fallback: String,
}

impl ConfigWarning {
    fn new(
        line: Option<usize>,
        key: impl Into<String>,
        value: impl Into<String>,
        fallback: impl Into<String>,
    ) -> Self {
        Self {
            line,
            key: key.into(),
            value: one_line(&value.into()),
            fallback: one_line(&fallback.into()),
        }
    }

    pub(crate) fn format(&self, path: &Path) -> String {
        let location = match self.line {
            Some(line) => format!("{}:{line}", path.display()),
            None => path.display().to_string(),
        };
        format!(
            "{location}: {}: rejected {}: using {}",
            self.key, self.value, self.fallback
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConfigLoad {
    pub(crate) config: Config,
    pub(crate) warnings: Vec<ConfigWarning>,
}

pub(crate) fn load(path: &Path) -> ConfigLoad {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ConfigLoad {
                config: Config::default(),
                warnings: Vec::new(),
            }
        }
        Err(error) => {
            return ConfigLoad {
                config: Config::default(),
                warnings: vec![ConfigWarning::new(
                    None,
                    "config",
                    error.to_string(),
                    "built-in defaults",
                )],
            }
        }
    };
    match Document::parse(text.as_str()) {
        Ok(document) => {
            let themes_dir = path.parent().map(|parent| parent.join("themes"));
            load_document(&text, &document, themes_dir.as_deref())
        }
        Err(error) => ConfigLoad {
            config: Config::default(),
            warnings: vec![ConfigWarning::new(
                line_at(&text, error.span()),
                "config",
                error.message(),
                "built-in defaults",
            )],
        },
    }
}

pub(crate) fn write_warning_log(log_path: &Path, warnings: &[String]) -> Result<()> {
    if warnings.is_empty() {
        return remove_stale_log(log_path);
    }
    let text = warnings.join("\n");
    write_private_atomically(log_path, format!("{text}\n").as_bytes())
}

fn remove_stale_log(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn load_document(text: &str, document: &Document<&str>, theme_dir: Option<&Path>) -> ConfigLoad {
    let mut loader = Loader {
        text,
        theme_dir,
        config: Config::default(),
        warnings: Vec::new(),
    };
    for (key, item) in document.iter() {
        loader.load_top_level(key, item);
    }
    ConfigLoad {
        config: loader.config,
        warnings: loader.warnings,
    }
}

struct Loader<'a> {
    text: &'a str,
    theme_dir: Option<&'a Path>,
    config: Config,
    warnings: Vec<ConfigWarning>,
}

impl Loader<'_> {
    fn load_top_level(&mut self, key: &str, item: &Item) {
        if key == "history" {
            self.load_history(item);
            return;
        }
        let section = match key {
            "ui" => ConfigSection::Ui,
            "editor" => ConfigSection::Editor,
            _ => {
                self.unknown(key, item);
                return;
            }
        };
        self.load_section(section, key, item);
    }

    fn load_history(&mut self, item: &Item) {
        if item.is_table() {
            self.load_section(ConfigSection::History, "history", item);
            return;
        }
        let spec = options::lookup("history").expect("history is registered");
        self.load_option(spec, "history", item);
    }

    fn load_section(&mut self, section: ConfigSection, name: &str, item: &Item) {
        let Some(table) = item.as_table() else {
            self.warn(
                item.span(),
                name,
                item.to_string(),
                "section ignored, expected a table",
            );
            return;
        };
        for (key, item) in table.iter() {
            let Some(spec) = spec_for(section, key) else {
                self.unknown(&format!("{name}.{key}"), item);
                continue;
            };
            let warning_key = format!("{name}.{key}");
            self.load_option(spec, &warning_key, item);
        }
    }

    fn load_option(&mut self, spec: &'static OptionSpec, key: &str, item: &Item) {
        let fallback = spec.default_value().display();
        let context = options::OptionContext {
            theme_dir: self.theme_dir,
        };
        match option_input(item) {
            Some(input) => match options::validate_spec(spec, input, &context) {
                Ok(value) => {
                    self.config.values.insert(spec.name, value);
                }
                Err(_) => {
                    self.config.values.insert(spec.name, spec.default_value());
                    self.warn(item.span(), key, item.to_string(), fallback);
                }
            },
            None => {
                self.warn(
                    item.span(),
                    key,
                    item.to_string(),
                    format!("{}, expected {}", fallback, expected_type(spec)),
                );
                self.config.values.insert(spec.name, spec.default_value());
            }
        }
    }

    fn unknown(&mut self, key: &str, item: &Item) {
        self.warn(item.span(), key, item.to_string(), "ignored");
    }

    fn warn(
        &mut self,
        span: Option<std::ops::Range<usize>>,
        key: impl Into<String>,
        value: impl Into<String>,
        fallback: impl Into<String>,
    ) {
        self.warnings.push(ConfigWarning::new(
            line_at(self.text, span),
            key,
            value,
            fallback,
        ));
    }
}

fn spec_for(section: ConfigSection, key: &str) -> Option<&'static OptionSpec> {
    options::OPTIONS.iter().find(|spec| {
        spec.section == section
            && (spec.config_key == key || spec.name == key || spec.aliases.contains(&key))
    })
}

fn option_input(item: &Item) -> Option<OptionInput<'_>> {
    match item.as_value()? {
        Value::Boolean(value) => Some(OptionInput::Bool(*value.value())),
        Value::Integer(value) => Some(OptionInput::Integer(*value.value())),
        Value::String(value) => Some(OptionInput::Text(value.value())),
        _ => None,
    }
}

fn expected_type(spec: &OptionSpec) -> &'static str {
    match spec.kind {
        options::OptionKind::Bool => "a boolean",
        options::OptionKind::Int { .. } => "an integer",
        options::OptionKind::Enum(_) => "a name",
    }
}

fn line_at(text: &str, span: Option<std::ops::Range<usize>>) -> Option<usize> {
    span.map(|span| {
        text[..span.start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use kawi_editor::{DEFAULT_HISTORY_LIMIT, MAX_HISTORY_LIMIT};
    use std::path::PathBuf;

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
autosave = "focus"
autosave_delay = 250

[history]
limit = 25
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
            loaded.config.get("autosave"),
            Some(&OptionValue::Text("focus".to_string()))
        );
        assert_eq!(
            loaded.config.get("autosave_delay"),
            Some(&OptionValue::Int(250))
        );
        assert_eq!(loaded.config.get("history"), Some(&OptionValue::Int(25)));
    }

    #[test]
    fn accepts_aliases_in_editor_section() {
        let loaded = load_text("[editor]\nsw = 2\net = true\n");
        assert!(loaded.warnings.is_empty());
        assert_eq!(loaded.config.get("shiftwidth"), Some(&OptionValue::Int(2)));
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
    fn custom_theme_files_are_valid_config_values() {
        let dir = temp_dir("custom_theme");
        let themes = dir.join("themes");
        fs::create_dir_all(&themes).unwrap();
        fs::write(
            themes.join("personal.toml"),
            crate::test_support::custom_theme(),
        )
        .unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "[ui]\ntheme = \"personal\"\n").unwrap();
        let loaded = load(&path);
        assert!(loaded.warnings.is_empty());
        assert_eq!(
            loaded.config.get("theme"),
            Some(&OptionValue::Text("personal".to_string()))
        );
        assert_eq!(
            kawi_tui::theme::Theme::load("personal", &themes)
                .unwrap()
                .name,
            "personal"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_custom_themes_fall_back_with_a_warning() {
        let dir = temp_dir("invalid_theme");
        let themes = dir.join("themes");
        fs::create_dir_all(&themes).unwrap();
        fs::write(
            themes.join("personal.toml"),
            "[base]\nforeground = \"#zzzzzz\"\n",
        )
        .unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "[ui]\ntheme = \"personal\"\n").unwrap();
        let loaded = load(&path);
        assert_eq!(
            loaded.config.get("theme"),
            Some(&OptionValue::Text("default".to_string()))
        );
        assert_eq!(loaded.warnings.len(), 1);
        assert!(loaded.warnings[0]
            .format(&path)
            .contains("ui.theme: rejected \"personal\": using default"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_custom_themes_fall_back_with_a_warning() {
        let dir = temp_dir("missing_theme");
        let path = dir.join("config.toml");
        fs::write(&path, "[ui]\ntheme = \"personal\"\n").unwrap();
        let loaded = load(&path);
        assert_eq!(
            loaded.config.get("theme"),
            Some(&OptionValue::Text("default".to_string()))
        );
        assert_eq!(loaded.warnings.len(), 1);
        let _ = fs::remove_dir_all(dir);
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
}
