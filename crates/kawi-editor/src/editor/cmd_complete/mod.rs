use std::fs;
use std::path::{Path, PathBuf};

use super::Editor;
use crate::command::COMMAND_NAMES;
use crate::mode::Mode;
use crate::paths;

mod help;
mod options;
mod plugin;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CompletionState {
    original: String,
    start: usize,
    candidates: Vec<String>,
    selected: Option<usize>,
}

impl CompletionState {
    pub(super) fn step(&mut self, backward: bool) {
        if self.candidates.len() == 1 {
            self.selected = Some(0);
            return;
        }
        self.selected = match (self.selected, backward) {
            (None, false) => Some(0),
            (None, true) => Some(self.candidates.len().saturating_sub(1)),
            (Some(0), true) => None,
            (Some(index), true) => Some(index.saturating_sub(1)),
            (Some(index), false) if index.saturating_add(1) >= self.candidates.len() => None,
            (Some(index), false) => Some(index.saturating_add(1)),
        };
    }

    pub(super) fn text(&self) -> &str {
        match self.selected {
            Some(index) => &self.candidates[index],
            None => "",
        }
    }

    pub(super) fn start(&self) -> usize {
        self.start
    }

    pub(super) fn original(&self) -> &str {
        &self.original
    }

    pub(super) fn items(&self) -> &[String] {
        &self.candidates
    }

    pub(super) fn selected(&self) -> Option<usize> {
        self.selected
    }

    fn descends_into_directory(&self) -> bool {
        self.selected.is_some() && self.text().ends_with('/')
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CompletionKind {
    File,
    Buffer,
}

impl Editor {
    pub(super) fn complete_command(&mut self, backward: bool) {
        if self.mode != Mode::Command {
            return;
        }
        if self
            .completion
            .as_ref()
            .is_some_and(CompletionState::descends_into_directory)
        {
            self.completion = None;
        } else if let Some(state) = self.completion.as_mut() {
            state.step(backward);
            self.apply_completion();
            return;
        }
        let Some((start, candidates)) = self.completion_candidates() else {
            return;
        };
        if candidates.is_empty() {
            return;
        }
        let mut state = CompletionState {
            original: self.command.text().to_string(),
            start,
            candidates,
            selected: None,
        };
        state.step(backward);
        self.completion = Some(state);
        self.apply_completion();
    }

    fn apply_completion(&mut self) {
        let Some(state) = self.completion.as_ref() else {
            return;
        };
        let text = match state.selected() {
            Some(_) => format!("{}{}", &state.original()[..state.start()], state.text()),
            None => state.original().to_string(),
        };
        self.command_history.mark_edited();
        self.command.replace(&text);
    }

    fn completion_candidates(&self) -> Option<(usize, Vec<String>)> {
        let text = self.command.text();
        if text == "plugin" {
            return plugin::candidates(&self.command_completion, text, 0, "");
        }
        let (start, pattern) = last_token(text);
        if start == 0 {
            return Some((0, matching(COMMAND_NAMES.iter().copied(), pattern)));
        }
        let command = text[..start].split_whitespace().next().unwrap_or("");
        match command {
            "set" => options::candidates(&self.command_completion, text, start, pattern),
            "h" | "help" => help::candidates(&self.command_completion, text, start, pattern),
            "plugin" => plugin::candidates(&self.command_completion, text, start, pattern),
            _ => {
                let candidates = match completion_kind(command)? {
                    CompletionKind::File => file_candidates(
                        pattern,
                        &current_dir(),
                        &paths::home_dir().unwrap_or_default(),
                    ),
                    CompletionKind::Buffer => self.buffer_candidates(pattern),
                };
                Some((start, candidates))
            }
        }
    }

    fn buffer_candidates(&self, pattern: &str) -> Vec<String> {
        self.buffers
            .numbers()
            .into_iter()
            .filter_map(|number| self.buffer_name(number))
            .filter(|name| *name != "[No Name]" && name.contains(pattern))
            .map(str::to_string)
            .collect()
    }
}

fn last_token(text: &str) -> (usize, &str) {
    let start = text.rfind(char::is_whitespace).map_or(0, |index| index + 1);
    (start, &text[start..])
}

fn completion_kind(command: &str) -> Option<CompletionKind> {
    let command = command.strip_suffix('!').unwrap_or(command);
    match command {
        "e" | "edit" | "w" | "write" => Some(CompletionKind::File),
        "b" | "buffer" | "bd" | "bdelete" => Some(CompletionKind::Buffer),
        _ => None,
    }
}

pub(super) fn matching<'a>(names: impl IntoIterator<Item = &'a str>, pattern: &str) -> Vec<String> {
    let mut matches: Vec<String> = names
        .into_iter()
        .filter(|name| name.starts_with(pattern))
        .map(str::to_string)
        .collect();
    matches.sort();
    matches.dedup();
    matches
}

fn file_candidates(pattern: &str, cwd: &Path, home: &Path) -> Vec<String> {
    let (dir_part, base) = match pattern.rfind('/') {
        Some(index) => (&pattern[..=index], &pattern[index + 1..]),
        None => ("", pattern),
    };
    let Ok(entries) = fs::read_dir(directory_path(dir_part, cwd, home)) else {
        return Vec::new();
    };
    let mut candidates: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') && !base.starts_with('.') {
                return None;
            }
            if !name.starts_with(base) {
                return None;
            }
            let suffix = if entry.path().is_dir() { "/" } else { "" };
            Some(format!("{dir_part}{name}{suffix}"))
        })
        .collect();
    candidates.sort();
    candidates
}

fn directory_path(dir_part: &str, cwd: &Path, home: &Path) -> PathBuf {
    if let Some(rest) = dir_part.strip_prefix("~/") {
        return home.join(rest);
    }
    if dir_part.is_empty() {
        return cwd.to_path_buf();
    }
    let path = Path::new(dir_part);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn current_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_default()
}

#[cfg(test)]
mod tests;
