use std::collections::BTreeSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_picker::{Picker, PickerOutcome, PickerRow, TextInput};
use kawi_plugin::{RpcError, UiConfirmParams, UiInputParams, UiPromptOutcome, UiSelectParams};
use kawi_tui::frame::PromptMenu;
use serde_json::Value;

use super::super::text::sanitize;
use super::validation::{
    optional_text, required_text, DEFAULT_INPUT_BYTES, MAX_INPUT_BYTES, MAX_MESSAGE_BYTES,
    MAX_PLACEHOLDER_BYTES, MAX_SELECT_OPTIONS, MAX_SELECT_OPTION_BYTES, MAX_TITLE_BYTES,
};

pub(super) struct PromptSession {
    pub(super) request: Value,
    pub(super) plugin: String,
    kind: PromptKind,
}

enum PromptKind {
    Input(InputPrompt),
    Select(Box<Picker>),
    Confirm(ConfirmPrompt),
}

struct InputPrompt {
    label: String,
    input: TextInput,
    max_length: usize,
}

struct ConfirmPrompt {
    label: String,
}

impl PromptSession {
    pub(super) fn input(
        plugin: &str,
        request: Value,
        params: UiInputParams,
    ) -> Result<Self, RpcError> {
        let title = required_text(&params.title, "title", MAX_TITLE_BYTES)?;
        let placeholder = optional_text(
            params.placeholder.as_deref(),
            "placeholder",
            MAX_PLACEHOLDER_BYTES,
        )?;
        let max_length = params
            .max_length
            .map(|value| value as usize)
            .unwrap_or(DEFAULT_INPUT_BYTES);
        if !(1..=MAX_INPUT_BYTES).contains(&max_length) {
            return Err(RpcError::invalid_params(format!(
                "max_length must be between 1 and {MAX_INPUT_BYTES}"
            )));
        }
        let value = params.value.as_deref().map(sanitize).unwrap_or_default();
        if value.chars().count() > max_length {
            return Err(RpcError::invalid_params("value exceeds max_length"));
        }
        let label = match placeholder {
            Some(placeholder) => format!("{title} ({placeholder})"),
            None => title,
        };
        let mut input = TextInput::new();
        input.set_text(&value);
        Ok(Self {
            request,
            plugin: plugin.to_string(),
            kind: PromptKind::Input(InputPrompt {
                label,
                input,
                max_length,
            }),
        })
    }

    pub(super) fn select(
        plugin: &str,
        request: Value,
        params: UiSelectParams,
    ) -> Result<Self, RpcError> {
        let title = required_text(&params.title, "title", MAX_TITLE_BYTES)?;
        if params.options.is_empty() {
            return Err(RpcError::invalid_params("options must not be empty"));
        }
        if params.options.len() > MAX_SELECT_OPTIONS {
            return Err(RpcError::invalid_params(format!(
                "options must not exceed {MAX_SELECT_OPTIONS}"
            )));
        }
        let rows = params
            .options
            .iter()
            .enumerate()
            .map(|(index, option)| select_row(option, index))
            .collect::<Result<Vec<_>, _>>()?;
        let mut seen = BTreeSet::new();
        if rows.iter().any(|row| !seen.insert(row.display.as_str())) {
            return Err(RpcError::invalid_params("options must be unique"));
        }
        let selected = params.selected.unwrap_or(0) as usize;
        if selected >= rows.len() {
            return Err(RpcError::invalid_params("selected is out of range"));
        }
        let mut picker = Picker::locations(title, rows);
        picker.select_symbol(selected as u64);
        Ok(Self {
            request,
            plugin: plugin.to_string(),
            kind: PromptKind::Select(Box::new(picker)),
        })
    }

    pub(super) fn confirm(
        plugin: &str,
        request: Value,
        params: UiConfirmParams,
    ) -> Result<Self, RpcError> {
        let title = required_text(&params.title, "title", MAX_TITLE_BYTES)?;
        let message = optional_text(params.message.as_deref(), "message", MAX_MESSAGE_BYTES)?;
        let label = match message {
            Some(message) => format!("{title}: {message}"),
            None => title,
        };
        Ok(Self {
            request,
            plugin: plugin.to_string(),
            kind: PromptKind::Confirm(ConfirmPrompt { label }),
        })
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<UiPromptOutcome> {
        match &mut self.kind {
            PromptKind::Input(prompt) => handle_input_key(prompt, key),
            PromptKind::Select(picker) => handle_select_key(picker, key),
            PromptKind::Confirm(_) => handle_confirm_key(key),
        }
    }

    pub(super) fn paste(&mut self, text: &str) {
        match &mut self.kind {
            PromptKind::Input(prompt) => insert_text(prompt, text),
            PromptKind::Select(picker) => picker.insert_text(text),
            PromptKind::Confirm(_) => {}
        }
    }

    pub(super) fn menu(&self) -> Option<PromptMenu<'_>> {
        match &self.kind {
            PromptKind::Input(prompt) => Some(PromptMenu {
                label: &prompt.label,
                value: prompt.input.text(),
                cursor: prompt.input.cursor(),
            }),
            PromptKind::Confirm(prompt) => Some(PromptMenu {
                label: &prompt.label,
                value: "",
                cursor: 0,
            }),
            PromptKind::Select(_) => None,
        }
    }

    pub(super) fn picker(&self) -> Option<&Picker> {
        match self.kind {
            PromptKind::Select(ref picker) => Some(picker),
            _ => None,
        }
    }
}

fn insert_text(prompt: &mut InputPrompt, text: &str) {
    for character in text.chars().filter(|character| !character.is_control()) {
        if prompt.input.text().chars().count() >= prompt.max_length {
            break;
        }
        prompt.input.insert_char(character);
    }
}

fn handle_input_key(prompt: &mut InputPrompt, key: KeyEvent) -> Option<UiPromptOutcome> {
    if is_cancel(key) {
        return Some(UiPromptOutcome::Canceled);
    }
    match key.code {
        KeyCode::Enter => return Some(UiPromptOutcome::Text(prompt.input.text().to_string())),
        KeyCode::Left => prompt.input.move_left(),
        KeyCode::Right => prompt.input.move_right(),
        KeyCode::Home => prompt.input.move_home(),
        KeyCode::End => prompt.input.move_end(),
        KeyCode::Backspace => prompt.input.backspace(),
        KeyCode::Delete => prompt.input.delete(),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            prompt.input.clear();
        }
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            prompt.input.delete_word();
        }
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                && prompt.input.text().chars().count() < prompt.max_length =>
        {
            prompt.input.insert_char(character);
        }
        _ => {}
    }
    None
}

fn handle_select_key(picker: &mut Picker, key: KeyEvent) -> Option<UiPromptOutcome> {
    match picker.handle_key(key) {
        PickerOutcome::Open(row) => Some(UiPromptOutcome::Selected(row.display)),
        PickerOutcome::Cancel => Some(UiPromptOutcome::Canceled),
        PickerOutcome::Consumed | PickerOutcome::QueryChanged => None,
    }
}

fn handle_confirm_key(key: KeyEvent) -> Option<UiPromptOutcome> {
    if is_cancel(key) {
        return Some(UiPromptOutcome::Canceled);
    }
    match key.code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => Some(UiPromptOutcome::Confirmed(true)),
        KeyCode::Char('n' | 'N') => Some(UiPromptOutcome::Confirmed(false)),
        _ => None,
    }
}

fn is_cancel(key: KeyEvent) -> bool {
    key.code == KeyCode::Esc
        || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
}

fn select_row(option: &str, index: usize) -> Result<PickerRow, RpcError> {
    let display = required_text(option, "option", MAX_SELECT_OPTION_BYTES)?;
    Ok(PickerRow {
        display,
        path: String::new(),
        line: None,
        column: None,
        buffer: None,
        secondary: None,
        deprecated: false,
        id: Some(index as u64),
    })
}
