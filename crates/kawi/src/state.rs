use anyhow::{Context, Result};
use kawi_editor::paths::home_dir;
use kawi_editor::{Editor, HistoryEntries, DEFAULT_HISTORY_LIMIT, MAX_HISTORY_LIMIT};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct StatePaths {
    pub config: PathBuf,
    pub languages: PathBuf,
    pub history: PathBuf,
}

impl StatePaths {
    pub fn resolve() -> Option<Self> {
        let config = xdg_dir("XDG_CONFIG_HOME", ".config")?;
        Some(Self {
            config: config.join("kawi/config.toml"),
            languages: config.join("kawi/languages.toml"),
            history: xdg_dir("XDG_STATE_HOME", ".local/state")?.join("kawi/history"),
        })
    }
}

pub fn load(editor: &mut Editor, paths: &StatePaths) -> Option<String> {
    let (config, warning) = load_config(&paths.config);
    editor.set_history_limit(config.history);
    editor.load_history(load_history(&paths.history));
    warning
}

pub fn save(editor: &Editor, paths: &StatePaths) -> Result<()> {
    save_history(&paths.history, &editor.history_entries())
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct Config {
    history: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            history: DEFAULT_HISTORY_LIMIT,
        }
    }
}

fn load_config(path: &Path) -> (Config, Option<String>) {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (Config::default(), None);
        }
        Err(error) => return (Config::default(), Some(format!("config.toml: {error}"))),
    };
    let config: Config = match toml::from_str(&text) {
        Ok(config) => config,
        Err(error) => return (Config::default(), Some(config_error(&error.to_string()))),
    };
    if config.history > MAX_HISTORY_LIMIT {
        let message = format!(
            "config.toml: E474: Invalid argument: history={}",
            config.history
        );
        return (Config::default(), Some(message));
    }
    (config, None)
}

fn config_error(details: &str) -> String {
    format!("config.toml: {}", one_line(details))
}

fn one_line(details: &str) -> String {
    details.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn load_history(path: &Path) -> HistoryEntries {
    let Ok(text) = fs::read_to_string(path) else {
        return HistoryEntries::default();
    };
    let mut entries = HistoryEntries::default();
    for line in text.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        match line.chars().next() {
            Some(':') => entries.command.push(line[1..].to_string()),
            Some('/') => entries.search.push(line[1..].to_string()),
            _ => {}
        }
    }
    entries
}

fn save_history(path: &Path, entries: &HistoryEntries) -> Result<()> {
    let mut text = String::new();
    append_entries(&mut text, ':', &entries.command);
    append_entries(&mut text, '/', &entries.search);
    write_atomically(path, &text)
}

fn append_entries(text: &mut String, marker: char, entries: &[String]) {
    for entry in entries {
        text.push(marker);
        text.push_str(entry);
        text.push('\n');
    }
}

fn write_atomically(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let temp = path.with_extension("tmp");
    fs::write(&temp, text).with_context(|| format!("failed to write {}", temp.display()))?;
    fs::rename(&temp, path).with_context(|| format!("failed to replace {}", path.display()))?;
    Ok(())
}

fn xdg_dir(variable: &str, fallback: &str) -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(variable) {
        let path = PathBuf::from(dir);
        if path.is_absolute() {
            return Some(path);
        }
    }
    Some(home_dir()?.join(fallback))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::Buffer;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kawi_state_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn editor() -> Editor {
        Editor::new(Buffer::from_string("text"))
    }

    #[test]
    fn config_defaults_when_missing() {
        let dir = temp_dir("config_missing");
        let (config, warning) = load_config(&dir.join("config.toml"));
        assert_eq!(config.history, DEFAULT_HISTORY_LIMIT);
        assert_eq!(warning, None);
    }

    #[test]
    fn config_reads_the_history_limit() {
        let dir = temp_dir("config_history");
        let path = dir.join("config.toml");
        fs::write(&path, "history = 25\n").unwrap();
        let (config, warning) = load_config(&path);
        assert_eq!(config.history, 25);
        assert_eq!(warning, None);
    }

    #[test]
    fn config_rejects_out_of_range_history() {
        let dir = temp_dir("config_range");
        let path = dir.join("config.toml");
        fs::write(&path, "history = 10001\n").unwrap();
        let (config, warning) = load_config(&path);
        assert_eq!(config.history, DEFAULT_HISTORY_LIMIT);
        assert_eq!(
            warning.as_deref(),
            Some("config.toml: E474: Invalid argument: history=10001")
        );
    }

    #[test]
    fn config_reports_parse_errors() {
        let dir = temp_dir("config_invalid");
        let path = dir.join("config.toml");
        fs::write(&path, "history = \n").unwrap();
        let (config, warning) = load_config(&path);
        assert_eq!(config.history, DEFAULT_HISTORY_LIMIT);
        let warning = warning.expect("parse warning");
        assert!(warning.starts_with("config.toml: "));
        assert!(!warning.contains('\n'));
    }

    #[test]
    fn history_round_trips_through_the_file() {
        let dir = temp_dir("history_roundtrip");
        let path = dir.join("history");
        let entries = HistoryEntries {
            command: vec!["write".to_string(), "echo \"quoted\"".to_string()],
            search: vec!["alpha".to_string(), "  spaced".to_string()],
        };
        save_history(&path, &entries).unwrap();
        assert_eq!(load_history(&path), entries);
    }

    #[test]
    fn history_load_ignores_unknown_lines() {
        let dir = temp_dir("history_unknown");
        let path = dir.join("history");
        fs::write(&path, "junk\n:keep\n\n/pattern\r\n").unwrap();
        let entries = load_history(&path);
        assert_eq!(entries.command, vec!["keep"]);
        assert_eq!(entries.search, vec!["pattern"]);
    }

    #[test]
    fn history_load_missing_file_is_empty() {
        let dir = temp_dir("history_missing");
        assert_eq!(
            load_history(&dir.join("history")),
            HistoryEntries::default()
        );
    }

    #[test]
    fn state_load_applies_config_and_history() {
        let dir = temp_dir("load_applies");
        let paths = StatePaths {
            config: dir.join("config.toml"),
            languages: dir.join("languages.toml"),
            history: dir.join("history"),
        };
        fs::write(&paths.config, "history = 1\n").unwrap();
        fs::write(&paths.history, ":first\n:second\n/search\n").unwrap();
        let mut editor = editor();
        let warning = load(&mut editor, &paths);
        assert_eq!(warning, None);
        let entries = editor.history_entries();
        assert_eq!(entries.command, vec!["second"]);
        assert_eq!(entries.search, vec!["search"]);
    }

    #[test]
    fn state_save_writes_the_editor_history() {
        let dir = temp_dir("save_editor");
        let paths = StatePaths {
            config: dir.join("config.toml"),
            languages: dir.join("languages.toml"),
            history: dir.join("history"),
        };
        let mut editor = editor();
        editor.load_history(HistoryEntries {
            command: vec!["write".to_string()],
            search: vec!["needle".to_string()],
        });
        save(&editor, &paths).unwrap();
        assert_eq!(
            fs::read_to_string(&paths.history).unwrap(),
            ":write\n/needle\n"
        );
    }
}
