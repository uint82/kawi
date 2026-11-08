use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use kawi_buffer::TextEdit;
use kawi_completion::Snippet;

use self::choice::SnippetChoiceOutcome;
pub(super) use self::choice::SnippetChoiceSession;
pub(super) use self::session::SnippetSession;
use self::session::SnippetStep;
use super::App;

mod choice;
mod session;

#[cfg(test)]
mod tests;

impl App {
    pub(crate) fn start_snippet(&mut self, snippet: Snippet) {
        self.end_snippet();
        let text_len = snippet.text().chars().count();
        let base = self.editor.cursor_char().saturating_sub(text_len);
        let snapshot = self.editor.document_snapshot();
        let Some(session) = SnippetSession::new(&snippet, base, snapshot) else {
            self.finish_snippet(base.saturating_add(snippet.final_stop()));
            return;
        };
        let range = session.active_range();
        self.editor.select_char_range(range.start, range.end);
        self.snippet = Some(session);
        self.open_snippet_choices();
    }

    pub(super) fn update_snippet(&mut self) {
        let snapshot = self.editor.document_snapshot();
        let Some(session) = self.snippet.as_mut() else {
            return;
        };
        session.update(&snapshot);
        let edits = session.mirror_edits(&snapshot);
        if edits.is_empty() {
            return;
        }
        let revision = self.editor.buffer_revision();
        if self
            .editor
            .apply_text_edits_at_revision(revision, &edits)
            .is_err()
        {
            return;
        }
        self.lsp.sync_editor(&self.editor);
        let snapshot = self.editor.document_snapshot();
        let Some(session) = self.snippet.as_mut() else {
            return;
        };
        session.update(&snapshot);
        let range = session.active_range();
        self.editor.select_char_range(range.start, range.end);
    }

    pub(super) fn end_snippet(&mut self) {
        self.snippet_choices = None;
        if self.snippet.take().is_some() {
            self.editor.clear_selection();
        }
    }

    pub(super) fn reselect_snippet_stop(&mut self) {
        let Some(range) = self.snippet.as_ref().map(SnippetSession::active_range) else {
            return;
        };
        self.editor.select_char_range(range.start, range.end);
    }

    fn finish_snippet(&mut self, position: usize) {
        self.snippet = None;
        self.editor.select_char_range(position, position);
        self.editor.clear_selection();
    }

    pub(super) fn handle_snippet_key(&mut self, key: KeyEvent) -> Option<bool> {
        self.snippet.as_ref()?;
        match key.code {
            KeyCode::Esc => {
                self.end_snippet();
                None
            }
            KeyCode::Tab
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.previous_snippet_stop();
                } else {
                    self.next_snippet_stop();
                }
                Some(false)
            }
            KeyCode::BackTab => {
                self.previous_snippet_stop();
                Some(false)
            }
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown => {
                self.end_snippet();
                None
            }
            _ => None,
        }
    }

    fn next_snippet_stop(&mut self) {
        let Some(step) = self.snippet.as_mut().map(SnippetSession::next) else {
            return;
        };
        match step {
            SnippetStep::Select(range) => {
                self.editor.select_char_range(range.start, range.end);
                self.open_snippet_choices();
            }
            SnippetStep::Finished(position) => self.finish_snippet(position),
        }
    }

    fn previous_snippet_stop(&mut self) {
        let Some(range) = self.snippet.as_mut().and_then(SnippetSession::previous) else {
            return;
        };
        self.editor.select_char_range(range.start, range.end);
        self.open_snippet_choices();
    }

    fn open_snippet_choices(&mut self) {
        self.snippet_choices = None;
        let Some(session) = self.snippet.as_ref() else {
            return;
        };
        let Some(choices) = session.active_choices().map(<[String]>::to_vec) else {
            return;
        };
        let range = session.active_range();
        let current = self
            .editor
            .document_snapshot()
            .slice(range.start..range.end)
            .to_string();
        self.snippet_choices = SnippetChoiceSession::new(&choices, &current);
    }

    pub(super) fn handle_snippet_choice_key(&mut self, key: KeyEvent) -> bool {
        let Some(mut session) = self.snippet_choices.take() else {
            return false;
        };
        match session.handle_key(key) {
            SnippetChoiceOutcome::Consumed => {
                self.snippet_choices = Some(session);
                true
            }
            SnippetChoiceOutcome::Cancel => true,
            SnippetChoiceOutcome::Apply => {
                self.accept_snippet_choice(&session);
                true
            }
            SnippetChoiceOutcome::Dismiss => false,
        }
    }

    fn accept_snippet_choice(&mut self, session: &SnippetChoiceSession) {
        let Some(choice) = session.selected_choice() else {
            return;
        };
        let Some(range) = self.snippet.as_ref().map(SnippetSession::active_range) else {
            return;
        };
        let edit = TextEdit {
            start: range.start,
            end: range.end,
            text: choice.to_string(),
        };
        let revision = self.editor.buffer_revision();
        if self
            .editor
            .apply_text_edits_at_revision(revision, &[edit])
            .is_err()
        {
            return;
        }
        self.lsp.sync_editor(&self.editor);
        self.update_snippet();
        self.reselect_snippet_stop();
    }
}
