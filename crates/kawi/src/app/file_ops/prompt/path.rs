use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_picker::TextInput;
use kawi_tui::frame::PromptMenu;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathAction {
    Rename { source: PathBuf, force: bool },
    NewFile { root: PathBuf },
    NewDirectory { root: PathBuf },
}

pub(crate) enum PathOutcome {
    Consumed,
    Cancel,
    Submit { action: PathAction, text: String },
}

pub(crate) struct PathPrompt {
    label: &'static str,
    input: TextInput,
    action: PathAction,
}

impl PathPrompt {
    pub(crate) fn rename(source: PathBuf, force: bool) -> Self {
        let text = source.to_string_lossy();
        let mut input = TextInput::new();
        input.set_text(&text);
        if let Some((start, end)) = stem_range(&text) {
            input.select_range(start, end);
        }
        Self {
            label: "rename",
            input,
            action: PathAction::Rename { source, force },
        }
    }

    pub(crate) fn new_file(root: PathBuf) -> Self {
        Self::new_path(root, "new file", |root| PathAction::NewFile { root })
    }

    pub(crate) fn new_directory(root: PathBuf) -> Self {
        Self::new_path(root, "new directory", |root| PathAction::NewDirectory {
            root,
        })
    }

    fn new_path(
        root: PathBuf,
        label: &'static str,
        action: impl FnOnce(PathBuf) -> PathAction,
    ) -> Self {
        let text = directory_text(&root);
        let mut input = TextInput::new();
        input.set_text(&text);
        Self {
            label,
            input,
            action: action(root),
        }
    }

    pub(crate) fn prompt(&self) -> PromptMenu<'_> {
        PromptMenu {
            label: self.label,
            value: self.input.text(),
            cursor: self.input.cursor(),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> PathOutcome {
        if key.code == KeyCode::Esc
            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
        {
            return PathOutcome::Cancel;
        }
        match key.code {
            KeyCode::Enter => {
                if self.input.text().is_empty() {
                    return PathOutcome::Consumed;
                }
                return PathOutcome::Submit {
                    action: self.action.clone(),
                    text: self.input.text().to_string(),
                };
            }
            KeyCode::Left => self.input.move_left(),
            KeyCode::Right => self.input.move_right(),
            KeyCode::Home => self.input.move_home(),
            KeyCode::End => self.input.move_end(),
            KeyCode::Backspace => self.input.backspace(),
            KeyCode::Delete => self.input.delete(),
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.clear();
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.delete_word();
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.input.insert_char(character);
            }
            _ => {}
        }
        PathOutcome::Consumed
    }

    pub(crate) fn paste(&mut self, text: &str) {
        self.input.insert_text(text);
    }
}

fn directory_text(path: &Path) -> String {
    let text = path.to_string_lossy();
    if text.ends_with(std::path::MAIN_SEPARATOR) {
        text.into_owned()
    } else {
        format!("{text}{}", std::path::MAIN_SEPARATOR)
    }
}

fn stem_range(text: &str) -> Option<(usize, usize)> {
    let separator = text.rfind(['/', '\\']).map_or(0, |index| index + 1);
    let file = &text[separator..];
    if file.is_empty() {
        return None;
    }
    let start = text[..separator].chars().count();
    let file_chars = file.chars().count();
    let end = match file.rfind('.').filter(|index| *index > 0) {
        Some(dot) => start + file[..dot].chars().count(),
        None => start + file_chars,
    };
    (start < end).then_some((start, end))
}

#[cfg(test)]
mod tests {
    use super::stem_range;

    #[test]
    fn selects_the_stem_before_the_extension() {
        assert_eq!(stem_range("src/main.rs"), Some((4, 8)));
    }

    #[test]
    fn selects_the_whole_name_without_an_extension() {
        assert_eq!(stem_range("src/Makefile"), Some((4, 12)));
    }
}
