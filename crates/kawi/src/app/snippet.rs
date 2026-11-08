use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use kawi_completion::Snippet;

pub(super) use self::session::SnippetSession;
use self::session::SnippetStep;
use super::App;

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
    }

    pub(super) fn update_snippet(&mut self) {
        let Some(session) = self.snippet.as_mut() else {
            return;
        };
        session.update(&self.editor.document_snapshot());
    }

    pub(super) fn end_snippet(&mut self) {
        if self.snippet.take().is_some() {
            self.editor.clear_selection();
        }
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
            SnippetStep::Select(range) => self.editor.select_char_range(range.start, range.end),
            SnippetStep::Finished(position) => self.finish_snippet(position),
        }
    }

    fn previous_snippet_stop(&mut self) {
        let Some(range) = self.snippet.as_mut().and_then(SnippetSession::previous) else {
            return;
        };
        self.editor.select_char_range(range.start, range.end);
    }
}
