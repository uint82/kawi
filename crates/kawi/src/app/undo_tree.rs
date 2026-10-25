use anyhow::Result;

use super::App;

impl App {
    pub(super) fn handle_undo_older(&mut self, forward: bool) -> Result<bool> {
        if self.editor.chronological_undo(forward) {
            self.reveal_cursor();
        } else {
            self.show_message(undo_boundary(forward).to_string());
        }
        Ok(false)
    }
}

pub(super) fn undo_boundary(forward: bool) -> &'static str {
    if forward {
        "Already at newest change"
    } else {
        "Already at oldest change"
    }
}

#[cfg(test)]
mod tests;
