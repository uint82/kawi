use anyhow::Result;
use kawi_editor::paths::home_dir;
use kawi_editor::{Editor, HistoryEntries};
use std::fs;
use std::path::{Path, PathBuf};

use crate::persist::write_atomically;

#[derive(Debug)]
pub struct StatePaths {
    pub config: PathBuf,
    pub config_log: PathBuf,
    pub languages: PathBuf,
    pub themes: PathBuf,
    pub history: PathBuf,
    pub sessions: PathBuf,
    pub recovery: PathBuf,
}

impl StatePaths {
    pub fn resolve() -> Option<Self> {
        let config = xdg_dir("XDG_CONFIG_HOME", ".config")?;
        let state = xdg_dir("XDG_STATE_HOME", ".local/state")?;
        Some(Self {
            config: config.join("kawi/config.toml"),
            config_log: state.join("kawi/config.log"),
            languages: config.join("kawi/languages.toml"),
            themes: config.join("kawi/themes"),
            history: state.join("kawi/history"),
            sessions: state.join("kawi/sessions"),
            recovery: state.join("kawi/recovery"),
        })
    }
}

pub fn load(editor: &mut Editor, paths: &StatePaths) {
    editor.load_history(load_history(&paths.history));
}

pub fn save(editor: &Editor, paths: &StatePaths) -> Result<()> {
    save_history(&paths.history, &editor.history_entries())
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

    fn paths(dir: &Path) -> StatePaths {
        StatePaths {
            config: dir.join("config.toml"),
            config_log: dir.join("config.log"),
            languages: dir.join("languages.toml"),
            themes: dir.join("themes"),
            history: dir.join("history"),
            sessions: dir.join("sessions"),
            recovery: dir.join("recovery"),
        }
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
    fn state_load_applies_history() {
        let dir = temp_dir("load_applies");
        let paths = paths(&dir);
        fs::write(&paths.history, ":first\n:second\n/search\n").unwrap();
        let mut editor = editor();
        load(&mut editor, &paths);
        let entries = editor.history_entries();
        assert_eq!(entries.command, vec!["first", "second"]);
        assert_eq!(entries.search, vec!["search"]);
    }

    #[test]
    fn state_save_writes_the_editor_history() {
        let dir = temp_dir("save_editor");
        let paths = paths(&dir);
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
