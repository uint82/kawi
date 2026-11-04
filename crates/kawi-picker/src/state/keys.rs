use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::TextInput;
use crate::selection::{selection_at, wrapped_selection};

use super::{Picker, PickerKind, PickerOutcome, PAGE_ROWS};

impl Picker {
    pub fn handle_key(&mut self, key: KeyEvent) -> PickerOutcome {
        match key.code {
            KeyCode::Esc => PickerOutcome::Cancel,
            KeyCode::Enter => self.open_selected(),
            KeyCode::Up => self.navigate(false, 1),
            KeyCode::Down => self.navigate(true, 1),
            KeyCode::PageUp => self.navigate(false, PAGE_ROWS),
            KeyCode::PageDown => self.navigate(true, PAGE_ROWS),
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => self.navigate(false, 1),
            KeyCode::BackTab => self.navigate(false, 1),
            KeyCode::Tab => self.navigate(true, 1),
            KeyCode::Home => self.set_selected(0),
            KeyCode::End => self.set_selected(self.rows.len().saturating_sub(1)),
            KeyCode::Left => self.move_query(false),
            KeyCode::Right => self.move_query(true),
            KeyCode::Backspace => self.edit_query(TextInput::backspace),
            KeyCode::Delete => self.edit_query(TextInput::delete),
            KeyCode::Char(character) => self.handle_character(key, character),
            _ => PickerOutcome::Consumed,
        }
    }

    fn handle_character(&mut self, key: KeyEvent, character: char) -> PickerOutcome {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match character {
                'c' => PickerOutcome::Cancel,
                'n' | 'j' => self.navigate(true, 1),
                'p' | 'k' => self.navigate(false, 1),
                'u' => self.edit_query(TextInput::clear),
                'w' => self.edit_query(TextInput::delete_word),
                _ => PickerOutcome::Consumed,
            };
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            return PickerOutcome::Consumed;
        }
        self.query.insert_char(character);
        self.after_query_change()
    }

    fn open_selected(&self) -> PickerOutcome {
        match self.selected.and_then(|index| self.rows.get(index)) {
            Some(row) => PickerOutcome::Open(row.clone()),
            None => PickerOutcome::Consumed,
        }
    }

    fn navigate(&mut self, forward: bool, amount: usize) -> PickerOutcome {
        self.selected = wrapped_selection(self.selected, self.rows.len(), forward, amount);
        PickerOutcome::Consumed
    }

    fn set_selected(&mut self, index: usize) -> PickerOutcome {
        self.selected = selection_at(index, self.rows.len());
        PickerOutcome::Consumed
    }

    fn move_query(&mut self, forward: bool) -> PickerOutcome {
        if forward {
            self.query.move_right();
        } else {
            self.query.move_left();
        }
        PickerOutcome::Consumed
    }

    fn edit_query(&mut self, edit: impl FnOnce(&mut TextInput)) -> PickerOutcome {
        edit(&mut self.query);
        self.after_query_change()
    }

    pub(super) fn after_query_change(&mut self) -> PickerOutcome {
        if self.kind == PickerKind::Grep {
            self.hits.clear();
            self.rows.clear();
            self.selected = None;
            self.status = None;
            self.loading = true;
            return PickerOutcome::QueryChanged;
        }
        if self.kind == PickerKind::WorkspaceSymbols {
            self.status = None;
            self.loading = true;
            return PickerOutcome::QueryChanged;
        }
        self.refresh();
        PickerOutcome::Consumed
    }
}
