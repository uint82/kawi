use std::fs;
use std::path::{Path, PathBuf};

use super::Editor;
use crate::command::{COMMAND_NAMES, SET_OPTION_NAMES};
use crate::mode::Mode;
use crate::paths;

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
    Option,
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
        let (start, pattern) = last_token(text);
        if start == 0 {
            return Some((0, matching(COMMAND_NAMES, pattern)));
        }
        let command = text[..start].split_whitespace().next().unwrap_or("");
        let kind = completion_kind(command)?;
        let candidates = match kind {
            CompletionKind::File => file_candidates(
                pattern,
                &current_dir(),
                &paths::home_dir().unwrap_or_default(),
            ),
            CompletionKind::Buffer => self.buffer_candidates(pattern),
            CompletionKind::Option => option_candidates(pattern),
        };
        Some((start, candidates))
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
        "set" => Some(CompletionKind::Option),
        _ => None,
    }
}

fn matching(names: &[&str], pattern: &str) -> Vec<String> {
    let mut matches: Vec<String> = names
        .iter()
        .filter(|name| name.starts_with(pattern))
        .map(|name| (*name).to_string())
        .collect();
    matches.sort();
    matches
}

fn option_candidates(pattern: &str) -> Vec<String> {
    let mut names: Vec<String> = SET_OPTION_NAMES
        .iter()
        .flat_map(|name| [(*name).to_string(), format!("no{name}")])
        .collect();
    names.retain(|name| name.starts_with(pattern));
    names.sort();
    names.dedup();
    names
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
mod tests {
    use super::*;
    use crate::actions::Action;
    use crate::command::parse;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn type_command(editor: &mut Editor, text: &str) {
        editor.apply(Action::EnterCommand);
        for ch in text.chars() {
            editor.apply(Action::PushCommandChar(ch));
        }
    }

    fn complete(editor: &mut Editor, backward: bool) {
        editor.complete_command(backward);
    }

    #[test]
    fn completes_command_names() {
        let mut e = editor_with("x");
        type_command(&mut e, "ed");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "edit");
        assert_eq!(
            parse(e.command_text()),
            crate::command::Command::Edit {
                name: None,
                force: false
            }
        );
    }

    #[test]
    fn unique_command_completion_stays() {
        let mut e = editor_with("x");
        type_command(&mut e, "edi");
        complete(&mut e, false);
        complete(&mut e, false);
        assert_eq!(e.command_text(), "edit");
    }

    #[test]
    fn cycles_commands_and_restores_original() {
        let mut e = editor_with("x");
        type_command(&mut e, "qu");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "quickfix");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "quit");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "quit!");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "quitall");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "quitall!");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "qu");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "quickfix");
    }

    #[test]
    fn backward_completion_from_original_selects_last() {
        let mut e = editor_with("x");
        type_command(&mut e, "qu");
        complete(&mut e, true);
        assert_eq!(e.command_text(), "quitall!");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "qu");
    }

    #[test]
    fn backward_from_first_returns_original() {
        let mut e = editor_with("x");
        type_command(&mut e, "qu");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "quickfix");
        complete(&mut e, true);
        assert_eq!(e.command_text(), "qu");
    }

    #[test]
    fn completes_option_names() {
        let mut e = editor_with("x");
        type_command(&mut e, "set nu");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "set number");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "set numberwidth");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "set nu");
    }

    #[test]
    fn completes_no_option_names() {
        let mut e = editor_with("x");
        type_command(&mut e, "set no");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "set noauto_completion");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "set noauto_pairs");
    }

    #[test]
    fn option_completion_does_not_cross_prefix() {
        let mut e = editor_with("x");
        type_command(&mut e, "set nu");
        complete(&mut e, false);
        complete(&mut e, false);
        assert_eq!(e.command_text(), "set numberwidth");
    }

    #[test]
    fn typing_clears_completion() {
        let mut e = editor_with("x");
        type_command(&mut e, "q");
        complete(&mut e, false);
        assert_eq!(e.command_text(), "q");
        e.apply(Action::PushCommandChar('!'));
        assert_eq!(e.completion, None);
    }

    #[test]
    fn cancel_clears_completion() {
        let mut e = editor_with("x");
        type_command(&mut e, "q");
        complete(&mut e, false);
        e.apply(Action::CancelCommand);
        assert_eq!(e.completion, None);
    }

    #[test]
    fn completes_buffer_names() {
        let mut e = editor_with("x");
        e.edit_file(Some("alpha.txt"), false).unwrap();
        e.edit_file(Some("beta.txt"), false).unwrap();
        type_command(&mut e, "b a");
        complete(&mut e, false);
        assert!(e.command_text().contains("alpha.txt") || e.command_text().contains("beta.txt"));
    }

    #[test]
    fn file_candidates_match_prefix_and_directories() {
        let dir = std::env::temp_dir().join("kawi_completion_files");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("alpha.txt"), "x").unwrap();
        fs::write(dir.join("alfred.txt"), "x").unwrap();
        fs::write(dir.join("beta.rs"), "x").unwrap();
        fs::write(dir.join(".hidden"), "x").unwrap();
        fs::write(dir.join("src/main.rs"), "x").unwrap();
        let home = Path::new("/nonexistent");
        assert_eq!(
            file_candidates("al", &dir, home),
            vec!["alfred.txt".to_string(), "alpha.txt".to_string()]
        );
        assert_eq!(file_candidates("sr", &dir, home), vec!["src/".to_string()]);
        assert_eq!(
            file_candidates("src/", &dir, home),
            vec!["src/main.rs".to_string()]
        );
        assert_eq!(
            file_candidates(".h", &dir, home),
            vec![".hidden".to_string()]
        );
        assert!(file_candidates("", &dir, home)
            .iter()
            .all(|name| !name.starts_with('.')));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn completing_directory_restarts_inside() {
        let dir = std::env::temp_dir().join("kawi_completion_dirs");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/main.rs"), "x").unwrap();
        fs::write(dir.join("src/lib.rs"), "x").unwrap();
        assert_eq!(
            file_candidates("sr", &dir, Path::new("/nonexistent")),
            vec!["src/".to_string()]
        );
        assert_eq!(
            file_candidates("src/", &dir, Path::new("/nonexistent")),
            vec!["src/lib.rs".to_string(), "src/main.rs".to_string()]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_candidates_use_home_for_tilde() {
        let home = std::env::temp_dir().join("kawi_completion_home");
        let _ = fs::remove_dir_all(&home);
        fs::create_dir_all(&home).unwrap();
        fs::write(home.join("homely.txt"), "x").unwrap();
        let candidates = file_candidates("~/hom", Path::new("/nonexistent"), &home);
        assert_eq!(candidates, vec!["~/homely.txt".to_string()]);
        let _ = fs::remove_dir_all(&home);
    }
}
