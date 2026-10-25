use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_tui::frame::PromptMenu;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WriteKind {
    SaveAs,
    WriteAs,
    CreateFile,
    CreateDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingWrite {
    pub name: String,
    pub force: bool,
    pub kind: WriteKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConfirmAction {
    CreateParents(PendingWrite),
}

pub(crate) enum ConfirmOutcome {
    Consumed,
    Cancel,
    Accept(ConfirmAction),
}

pub(crate) struct ConfirmSession {
    label: String,
    action: ConfirmAction,
}

impl ConfirmSession {
    pub(crate) fn create_parents(label: String, action: PendingWrite) -> Self {
        Self {
            label,
            action: ConfirmAction::CreateParents(action),
        }
    }

    pub(crate) fn prompt(&self) -> PromptMenu<'_> {
        PromptMenu {
            label: &self.label,
            value: "",
            cursor: 0,
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ConfirmOutcome {
        if key.code == KeyCode::Esc
            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
        {
            return ConfirmOutcome::Cancel;
        }
        match key.code {
            KeyCode::Char('y' | 'Y') => ConfirmOutcome::Accept(self.action.clone()),
            KeyCode::Char('n' | 'N') | KeyCode::Enter => ConfirmOutcome::Cancel,
            _ => ConfirmOutcome::Consumed,
        }
    }
}
