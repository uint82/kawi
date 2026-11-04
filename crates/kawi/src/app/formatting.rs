use std::path::{Path, PathBuf};

use kawi_buffer::TextEdit;
use kawi_lsp::{CursorPosition, FormatterConfig, FormattingEvent};

use super::formatter::FormatError;
use super::formatter_service::{FormatResult, FormatterService};
use super::lsp_navigation::cursor;
use super::App;

pub(super) struct PendingFormat {
    request: u64,
    number: usize,
    path: PathBuf,
    original_hash: u64,
}

impl App {
    pub(super) fn request_format(&mut self) -> anyhow::Result<bool> {
        if self.pending_format.is_some() {
            self.show_message("Format already in progress".to_string());
            return Ok(false);
        }
        if self.save_is_pending() {
            self.show_message("Save already in progress".to_string());
            return Ok(false);
        }
        if let Some((path, formatter)) = self.external_formatter() {
            let number = self.editor.buffer_number();
            if let Some(text) = self.editor.buffer_text(number) {
                if self.start_external_format(number, path, formatter, text) {
                    return Ok(false);
                }
            }
        }
        self.request_lsp_format();
        Ok(false)
    }

    pub(super) fn queue_formatter_request(
        &mut self,
        path: PathBuf,
        formatter: FormatterConfig,
        text: String,
    ) -> Result<u64, String> {
        if self.formatter_service.is_none() {
            self.formatter_service = Some(FormatterService::new(self.events.wake())?);
        }
        self.formatter_service
            .as_mut()
            .ok_or_else(|| "formatter service is unavailable".to_string())?
            .request(path, formatter, text)
    }

    fn request_lsp_format(&mut self) {
        let range = self.formatting_range();
        if let Err(message) = self.lsp.formatting(&self.editor, range) {
            self.show_message(message);
        }
    }

    fn start_external_format(
        &mut self,
        number: usize,
        path: PathBuf,
        formatter: FormatterConfig,
        text: String,
    ) -> bool {
        let Some(original_hash) = self.editor.file_hash(&path) else {
            return false;
        };
        match self.queue_formatter_request(path.clone(), formatter, text) {
            Ok(request) => {
                self.pending_format = Some(PendingFormat {
                    request,
                    number,
                    path,
                    original_hash,
                });
                true
            }
            Err(error) => {
                self.show_message(error);
                false
            }
        }
    }

    pub(super) fn poll_formatter(&mut self) -> bool {
        let mut results = Vec::new();
        if let Some(service) = self.formatter_service.as_ref() {
            while let Some(result) = service.try_recv() {
                results.push(result);
            }
        }
        results.into_iter().fold(false, |changed, result| {
            self.handle_formatter_result(result) || changed
        })
    }

    fn handle_formatter_result(&mut self, result: FormatResult) -> bool {
        if self
            .pending_format
            .as_ref()
            .is_some_and(|pending| pending.request == result.request)
        {
            self.finish_external_format(result);
            return true;
        }
        false
    }

    fn finish_external_format(&mut self, result: FormatResult) {
        let Some(pending) = self.pending_format.take() else {
            return;
        };
        if pending.request != result.request {
            self.pending_format = Some(pending);
            return;
        }
        match result.result {
            Ok(formatted) => self.apply_external_format(pending, formatted),
            Err(FormatError::NotFound(message)) => {
                self.show_message(format!("{message}; using language server"));
                self.request_lsp_format();
            }
            Err(FormatError::Failed(message)) => self.show_message(message),
        }
    }

    fn apply_external_format(&mut self, pending: PendingFormat, formatted: String) {
        if self.editor.file_hash(&pending.path) != Some(pending.original_hash) {
            self.show_message("Buffer changed while formatting".to_string());
            return;
        }
        let Some(current) = self.editor.buffer_text(pending.number) else {
            return;
        };
        if formatted != current {
            if let Err(message) = self.apply_formatted_text(&pending.path, &formatted) {
                self.show_message(message);
                return;
            }
        }
        self.show_message("Formatted".to_string());
        self.reveal_cursor();
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
    use std::time::{Duration, Instant};

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

    fn wait_for_formatter(app: &mut App) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && app.pending_format.is_some() {
            app.poll_formatter();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(app.pending_format.is_none(), "formatter timed out");
    }

    #[cfg(unix)]
    #[test]
    fn external_formatter_updates_the_buffer() {
        let (dir, mut app) = app_with_formatter("external_format", "sh");
        assert!(!app.request_format().unwrap());
        assert!(app.pending_format.is_some());
        assert_eq!(app.editor.buffer_text(1).as_deref(), Some("fn main() {}\n"));
        wait_for_formatter(&mut app);
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
        wait_for_formatter(&mut app);
        assert_eq!(app.editor.buffer_text(1).as_deref(), Some("fn main() {}\n"));
        assert!(app
            .last_message()
            .is_some_and(|message| message.contains("formatter exited")));
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn external_formatter_discards_a_changed_buffer() {
        let (dir, mut app) = app_with_formatter("external_format_stale", "sh");
        let config = dir.join("languages.toml");
        fs::write(
            &config,
            "[[language]]\nname = \"rust\"\nservers = []\nformatter = { command = \"sh\", args = [\"-c\", \"sleep 0.2; tr a-z A-Z\"] }\n",
        )
        .unwrap();
        app.lsp.load(&config, app.events.wake());
        assert!(!app.request_format().unwrap());
        app.editor.apply(Action::EnterInsert);
        app.editor.apply(Action::InsertChar('x'));
        app.editor.apply(Action::ExitInsert);
        wait_for_formatter(&mut app);
        assert_eq!(
            app.editor.buffer_text(1).as_deref(),
            Some("xfn main() {}\n")
        );
        assert!(app
            .last_message()
            .is_some_and(|message| message.contains("Buffer changed")));
        let _ = fs::remove_dir_all(dir);
    }
}
