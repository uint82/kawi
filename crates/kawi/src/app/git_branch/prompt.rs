use anyhow::Result;
use crossterm::event::KeyEvent;
use kawi_tui::frame::PromptMenu;

use crate::app::rename::{RenameOutcome, RenameSession};
use crate::app::App;

pub(crate) struct BranchPrompt {
    session: RenameSession,
    base: Option<String>,
}

impl BranchPrompt {
    pub(crate) fn new(base: Option<String>) -> Self {
        Self {
            session: RenameSession::input("branch"),
            base,
        }
    }

    pub(crate) fn prompt(&self) -> Option<PromptMenu<'_>> {
        self.session.prompt()
    }
}

impl App {
    pub(crate) fn open_branch_prompt(&mut self, base: Option<String>) {
        self.branch_prompt = Some(BranchPrompt::new(base));
    }

    pub(crate) fn handle_branch_prompt_key(&mut self, key: KeyEvent) -> Result<bool> {
        let Some(mut prompt) = self.branch_prompt.take() else {
            return Ok(false);
        };
        match prompt.session.handle_key(key) {
            RenameOutcome::Cancel => {}
            RenameOutcome::Consumed => self.branch_prompt = Some(prompt),
            RenameOutcome::Submit { name, .. } => {
                self.create_branch(&name, prompt.base.as_deref())?;
            }
        }
        Ok(false)
    }

    pub(crate) fn paste_branch_prompt(&mut self, text: &str) -> bool {
        let Some(prompt) = self.branch_prompt.as_mut() else {
            return false;
        };
        prompt.session.paste(text);
        true
    }
}
