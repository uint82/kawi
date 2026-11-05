use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::state::{ColorPicker, Mode};

const COARSE_MODIFIER: KeyModifiers = KeyModifiers::SHIFT;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerAction {
    Handled,
    Close,
    Request,
    Apply,
    CancelRequest,
}

impl ColorPicker {
    fn handle_key(&mut self, key: KeyEvent) -> PickerAction {
        self.clear_message();
        match self.mode() {
            Mode::Adjust => self.handle_adjust_key(key),
            Mode::Loading => self.handle_loading_key(key),
            Mode::Present => self.handle_present_key(key),
        }
    }

    fn handle_adjust_key(&mut self, key: KeyEvent) -> PickerAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => PickerAction::Close,
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_field(false);
                PickerAction::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_field(true);
                PickerAction::Handled
            }
            KeyCode::Left | KeyCode::Char('h') => self.adjust_key(false, is_coarse(key)),
            KeyCode::Right | KeyCode::Char('l') => self.adjust_key(true, is_coarse(key)),
            KeyCode::Char('H') => self.adjust_key(false, true),
            KeyCode::Char('L') => self.adjust_key(true, true),
            KeyCode::Enter => PickerAction::Request,
            _ => PickerAction::Handled,
        }
    }

    fn adjust_key(&mut self, forward: bool, coarse: bool) -> PickerAction {
        if forward {
            self.adjust(coarse);
        } else {
            self.decrease(coarse);
        }
        PickerAction::Handled
    }

    fn handle_loading_key(&mut self, key: KeyEvent) -> PickerAction {
        match key.code {
            KeyCode::Esc => PickerAction::CancelRequest,
            _ => PickerAction::Handled,
        }
    }

    fn handle_present_key(&mut self, key: KeyEvent) -> PickerAction {
        match key.code {
            KeyCode::Esc => {
                self.back_to_adjust();
                PickerAction::Handled
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_presentation(false);
                PickerAction::Handled
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_presentation(true);
                PickerAction::Handled
            }
            KeyCode::Enter => PickerAction::Apply,
            _ => PickerAction::Handled,
        }
    }
}

fn is_coarse(key: KeyEvent) -> bool {
    key.modifiers.contains(COARSE_MODIFIER)
}

impl super::super::App {
    pub(crate) fn handle_color_picker_key(&mut self, key: KeyEvent) -> anyhow::Result<bool> {
        let action = match self.color_picker.as_mut() {
            Some(picker) => picker.handle_key(key),
            None => return Ok(false),
        };
        match action {
            PickerAction::Handled => {}
            PickerAction::Close => self.close_color_picker(),
            PickerAction::Request => self.request_color_presentations(),
            PickerAction::Apply => self.apply_selected_color_presentation(),
            PickerAction::CancelRequest => self.cancel_color_presentation_request(),
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::super::state::{ColorPicker, Field, Mode};
    use super::PickerAction;
    use kawi_lsp::{CursorPosition, DocumentColor};
    use std::path::PathBuf;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn picker() -> ColorPicker {
        ColorPicker::new(
            &DocumentColor {
                start: CursorPosition { line: 0, column: 0 },
                end: CursorPosition { line: 0, column: 7 },
                red: 204,
                green: 34,
                blue: 51,
                alpha: 255,
            },
            PathBuf::from("/tmp/color.css"),
            1,
        )
    }

    #[test]
    fn adjust_keys_change_fields_and_values() {
        let mut picker = picker();
        assert_eq!(picker.handle_key(key(KeyCode::Down)), PickerAction::Handled);
        assert_eq!(picker.field(), Field::Saturation);
        assert_eq!(picker.handle_key(key(KeyCode::Left)), PickerAction::Handled);
        assert_eq!(
            picker.handle_key(key(KeyCode::Char('H'))),
            PickerAction::Handled
        );
        assert_eq!(picker.field(), Field::Saturation);
    }

    #[test]
    fn enter_requests_formats_and_loading_escape_cancels() {
        let mut picker = picker();
        assert_eq!(
            picker.handle_key(key(KeyCode::Enter)),
            PickerAction::Request
        );
        picker.begin_request(4);
        assert_eq!(picker.mode(), Mode::Loading);
        assert_eq!(
            picker.handle_key(key(KeyCode::Esc)),
            PickerAction::CancelRequest
        );
    }

    #[test]
    fn formats_navigate_and_escape_returns_to_adjust() {
        let mut picker = picker();
        picker.begin_request(4);
        let event = kawi_lsp::ColorPresentationEvent {
            request: 4,
            path: picker.path().clone(),
            presentations: vec![
                kawi_lsp::ColorPresentationEntry {
                    label: "#cc2233".to_string(),
                    edit: kawi_lsp::TextChange {
                        start: 0,
                        end: 7,
                        text: "#cc2233".to_string(),
                    },
                    additional: Vec::new(),
                },
                kawi_lsp::ColorPresentationEntry {
                    label: "rgb(204, 34, 51)".to_string(),
                    edit: kawi_lsp::TextChange {
                        start: 0,
                        end: 7,
                        text: "rgb(204, 34, 51)".to_string(),
                    },
                    additional: Vec::new(),
                },
            ],
        };
        assert!(picker.apply_event(event));
        assert_eq!(picker.mode(), Mode::Present);
        assert_eq!(picker.handle_key(key(KeyCode::Down)), PickerAction::Handled);
        assert_eq!(picker.selected(), 1);
        assert_eq!(picker.handle_key(key(KeyCode::Esc)), PickerAction::Handled);
        assert_eq!(picker.mode(), Mode::Adjust);
    }
}
