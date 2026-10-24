use kawi_lsp::{CursorPosition, FormattingEvent};

use super::lsp_navigation::cursor;
use super::App;

impl App {
    pub(super) fn request_format(&mut self) -> anyhow::Result<bool> {
        let range = self.formatting_range();
        match self.lsp.formatting(&self.editor, range) {
            Ok(_) => {}
            Err(message) => self.show_message(message),
        }
        Ok(false)
    }

    fn formatting_range(&self) -> Option<(CursorPosition, CursorPosition)> {
        let ((start_row, _), (end_row, _)) = self.editor.selection_range()?;
        let end_column = self.editor.line_len(end_row);
        Some((cursor(start_row, 0), cursor(end_row, end_column)))
    }

    pub(super) fn handle_formatting_event(&mut self, event: FormattingEvent) {
        match self.apply_workspace_edit(&event.edit) {
            Ok(_) => {
                self.show_message("Formatted".to_string());
                self.reveal_cursor();
            }
            Err(message) => self.show_message(message),
        }
    }

    pub(super) fn handle_formatting_result(&mut self) {
        if let Some(event) = self.lsp.take_formatting() {
            if self
                .pending_save
                .as_ref()
                .is_some_and(|pending| pending.matches(event.request))
            {
                self.finish_formatted_save(event);
            } else {
                self.handle_formatting_event(event);
            }
        }
        if let Some((request, message)) = self.lsp.take_formatting_failure() {
            self.finish_formatting_failure(request, message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;
    use kawi_editor::Action;

    #[test]
    fn document_formatting_has_no_range() {
        let app = App::new(Buffer::from_string("one\ntwo\n"), Flavor::Vim);
        assert_eq!(app.formatting_range(), None);
    }

    #[test]
    fn visual_selection_formats_whole_lines() {
        let mut app = App::new(Buffer::from_string("one\ntwo\n"), Flavor::Vim);
        app.editor.apply(Action::EnterVisualLine);
        app.editor.apply(Action::MoveDown);
        let range = app.formatting_range().expect("range");
        assert_eq!(range.0.line, 0);
        assert_eq!(range.0.column, 0);
        assert_eq!(range.1.line, 1);
        assert_eq!(range.1.column, 3);
    }
}
