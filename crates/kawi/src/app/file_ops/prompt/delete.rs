use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_picker::TextInput;
use kawi_tui::frame::PromptMenu;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeleteRequest {
    pub target: PathBuf,
    pub permanent: bool,
    pub dirty: Vec<usize>,
    pub items: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeleteStage {
    Confirm,
    Typed,
    Dirty,
}

pub(crate) enum DeleteOutcome {
    Consumed,
    Cancel,
    Execute { keep_dirty: bool },
}

pub(crate) struct DeleteSession {
    request: DeleteRequest,
    stage: DeleteStage,
    input: TextInput,
    label: String,
}

impl DeleteSession {
    pub(crate) fn new(request: DeleteRequest, requires_typed: bool) -> Self {
        let stage = if requires_typed {
            DeleteStage::Typed
        } else {
            DeleteStage::Confirm
        };
        let mut session = Self {
            request,
            stage,
            input: TextInput::new(),
            label: String::new(),
        };
        session.refresh_label();
        session
    }

    pub(crate) fn request(&self) -> &DeleteRequest {
        &self.request
    }

    pub(crate) fn prompt(&self) -> PromptMenu<'_> {
        let value = if self.stage == DeleteStage::Typed {
            self.input.text()
        } else {
            ""
        };
        let cursor = if self.stage == DeleteStage::Typed {
            self.input.cursor()
        } else {
            0
        };
        PromptMenu {
            label: &self.label,
            value,
            cursor,
        }
    }

    fn refresh_label(&mut self) {
        let target = self.request.target.to_string_lossy();
        self.label = match self.stage {
            DeleteStage::Confirm => {
                let action = if self.request.permanent {
                    "Permanently delete"
                } else {
                    "Trash"
                };
                let items = self
                    .request
                    .items
                    .map(|items| format!(" ({items} items)"))
                    .unwrap_or_default();
                format!("{action} \"{target}\"{items}? [y/N]")
            }
            DeleteStage::Typed => {
                let items = self
                    .request
                    .items
                    .map(|items| format!(" ({items} items)"))
                    .unwrap_or_default();
                format!("Type delete to confirm \"{target}\"{items}")
            }
            DeleteStage::Dirty => format!(
                "Keep {} dirty {} as unsaved? [k]eep [d]iscard [c]ancel",
                self.request.dirty.len(),
                if self.request.dirty.len() == 1 {
                    "buffer"
                } else {
                    "buffers"
                }
            ),
        };
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> DeleteOutcome {
        if key.code == KeyCode::Esc
            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
        {
            return DeleteOutcome::Cancel;
        }
        match self.stage {
            DeleteStage::Confirm => self.handle_confirm(key),
            DeleteStage::Typed => self.handle_typed(key),
            DeleteStage::Dirty => self.handle_dirty(key),
        }
    }

    fn handle_confirm(&mut self, key: KeyEvent) -> DeleteOutcome {
        match key.code {
            KeyCode::Char('y' | 'Y') => self.advance(),
            KeyCode::Char('n' | 'N') | KeyCode::Enter => DeleteOutcome::Cancel,
            _ => DeleteOutcome::Consumed,
        }
    }

    fn handle_typed(&mut self, key: KeyEvent) -> DeleteOutcome {
        match key.code {
            KeyCode::Enter => {
                if self.input.text() == "delete" {
                    self.advance()
                } else {
                    DeleteOutcome::Consumed
                }
            }
            KeyCode::Backspace => {
                self.input.backspace();
                DeleteOutcome::Consumed
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.clear();
                DeleteOutcome::Consumed
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.input.insert_char(character);
                DeleteOutcome::Consumed
            }
            _ => DeleteOutcome::Consumed,
        }
    }

    fn handle_dirty(&mut self, key: KeyEvent) -> DeleteOutcome {
        match key.code {
            KeyCode::Char('k' | 'K') | KeyCode::Enter => {
                DeleteOutcome::Execute { keep_dirty: true }
            }
            KeyCode::Char('d' | 'D') => DeleteOutcome::Execute { keep_dirty: false },
            KeyCode::Char('c' | 'C') => DeleteOutcome::Cancel,
            _ => DeleteOutcome::Consumed,
        }
    }

    fn advance(&mut self) -> DeleteOutcome {
        if self.request.dirty.is_empty() {
            return DeleteOutcome::Execute { keep_dirty: false };
        }
        self.stage = DeleteStage::Dirty;
        self.refresh_label();
        DeleteOutcome::Consumed
    }

    pub(crate) fn paste(&mut self, text: &str) {
        if self.stage == DeleteStage::Typed {
            self.input.insert_text(text);
        }
    }
}
