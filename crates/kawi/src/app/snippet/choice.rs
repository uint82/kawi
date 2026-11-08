use crossterm::event::KeyEvent;

use super::super::lsp_menu::{menu_key, MenuKey, MenuSelection};

pub(crate) enum SnippetChoiceOutcome {
    Consumed,
    Cancel,
    Apply,
    Dismiss,
}

pub(crate) struct SnippetChoiceSession {
    choices: Vec<String>,
    selection: MenuSelection,
}

impl SnippetChoiceSession {
    pub(crate) fn new(choices: &[String], current: &str) -> Option<Self> {
        if choices.len() < 2 {
            return None;
        }
        let preferred = choices.iter().position(|choice| choice == current);
        Some(Self {
            choices: choices.to_vec(),
            selection: MenuSelection::new(choices.len(), preferred),
        })
    }

    pub(crate) fn items(&self) -> &[String] {
        &self.choices
    }

    pub(crate) fn selected(&self) -> Option<usize> {
        self.selection.selected()
    }

    pub(crate) fn selected_choice(&self) -> Option<&str> {
        let index = self.selection.selected()?;
        self.choices.get(index).map(String::as_str)
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> SnippetChoiceOutcome {
        match menu_key(key) {
            MenuKey::Cancel => SnippetChoiceOutcome::Cancel,
            MenuKey::Move(delta) => {
                self.selection.move_by(delta);
                SnippetChoiceOutcome::Consumed
            }
            MenuKey::Accept => SnippetChoiceOutcome::Apply,
            MenuKey::Other => SnippetChoiceOutcome::Dismiss,
        }
    }
}
