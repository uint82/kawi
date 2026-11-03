use std::path::{Path, PathBuf};

use kawi_buffer::TextEdit;
use kawi_lsp::{CursorPosition, FormatterConfig, FormattingEvent};

use super::formatter::{self, FormatError};
use super::lsp_navigation::cursor;
use super::App;

impl App {
    pub(super) fn request_format(&mut self) -> anyhow::Result<bool> {
        match self.format_external() {
            Ok(true) => return Ok(false),
            Ok(false) => {}
            Err(message) => {
                self.show_message(message);
                return Ok(false);
            }
        }
        let range = self.formatting_range();
        match self.lsp.formatting(&self.editor, range) {
            Ok(_) => {}
            Err(message) => self.show_message(message),
        }
        Ok(false)
    }

    fn format_external(&mut self) -> Result<bool, String> {
        let Some((path, formatter)) = self.external_formatter() else {
            return Ok(false);
        };
        let Some(text) = self.editor.buffer_text(self.editor.buffer_number()) else {
            return Ok(false);
        };
        match formatter::format(&formatter, &path, &text) {
            Ok(formatted) => {
                if formatted != text {
                    self.apply_formatted_text(&path, &formatted)?;
                }
                self.show_message("Formatted".to_string());
                self.reveal_cursor();
                Ok(true)
            }
            Err(FormatError::NotFound(message)) => {
                self.show_message(format!("{message}; using language server"));
                Ok(false)
            }
            Err(FormatError::Failed(message)) => Err(message),
        }
    }

    pub(super) fn external_formatter(&self) -> Option<(PathBuf, FormatterConfig)> {
        if !self.editor.has_file() || self.editor.current_buffer_is_directory() {
            return None;
        }
        let path = PathBuf::from(self.editor.filename());
        self.lsp
            .formatter(Some(&path))
            .map(|formatter| (path, formatter))
    }

    pub(super) fn apply_formatted_text(&mut self, path: &Path, text: &str) -> Result<(), String> {
        let end = self
            .editor
            .file_char_len(path)
            .ok_or_else(|| format!("Buffer is not open: {}", path.display()))?;
        let edit = TextEdit {
            start: 0,
            end,
            text: text.to_string(),
        };
        self.editor
            .apply_text_edits(path, &[edit])
            .map_err(super::file_error_message)
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
    use crate::test_support::temp_dir;
    use kawi_buffer::Buffer;
    use kawi_editor::Action;
    use std::fs;

    fn app_with_formatter(name: &str, command: &str) -> (PathBuf, App) {
        let dir = temp_dir(name);
        let path = dir.join("main.rs");
        let mut buffer = Buffer::from_string("fn main() {}\n");
        buffer.path = Some(path.to_string_lossy().into_owned());
        let mut app = App::new(buffer, Flavor::Vim);
        let config = dir.join("languages.toml");
        fs::write(
            &config,
            format!(
                "[[language]]\nname = \"rust\"\nservers = []\nformatter = {{ command = {command:?}, args = [\"-c\", \"tr a-z A-Z\"] }}\n"
            ),
        )
        .unwrap();
        app.lsp.load(&config, app.events.wake());
        (dir, app)
    }

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

    #[cfg(unix)]
    #[test]
    fn external_formatter_updates_the_buffer() {
        let (dir, mut app) = app_with_formatter("external_format", "sh");
        assert!(!app.request_format().unwrap());
        assert_eq!(app.editor.buffer_text(1).as_deref(), Some("FN MAIN() {}\n"));
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn external_formatter_failure_keeps_the_buffer() {
        let (dir, mut app) = app_with_formatter("external_format_failure", "sh");
        let config = dir.join("languages.toml");
        fs::write(
            &config,
            "[[language]]\nname = \"rust\"\nservers = []\nformatter = { command = \"sh\", args = [\"-c\", \"exit 1\"] }\n",
        )
        .unwrap();
        app.lsp.load(&config, app.events.wake());
        assert!(!app.request_format().unwrap());
        assert_eq!(app.editor.buffer_text(1).as_deref(), Some("fn main() {}\n"));
        assert!(app
            .last_message()
            .is_some_and(|message| message.contains("formatter exited")));
        let _ = fs::remove_dir_all(dir);
    }
}
