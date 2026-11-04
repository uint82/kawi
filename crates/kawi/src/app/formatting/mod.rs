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

struct ManualFormat {
    number: usize,
    path: PathBuf,
    formatter: FormatterConfig,
    text: String,
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
                let format = ManualFormat {
                    number,
                    path,
                    formatter,
                    text,
                };
                if self.start_external_format(format) {
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

    fn start_external_format(&mut self, format: ManualFormat) -> bool {
        let Some(original_hash) = self.editor.file_hash(&format.path) else {
            return false;
        };
        match self.queue_formatter_request(format.path.clone(), format.formatter, format.text) {
            Ok(request) => {
                self.pending_format = Some(PendingFormat {
                    request,
                    number: format.number,
                    path: format.path,
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
        let mut changed = false;
        for result in results {
            changed |= self.handle_formatter_result(result);
        }
        changed
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
        if self
            .pending_save
            .as_ref()
            .and_then(super::save::PendingSave::external_request)
            == Some(result.request)
        {
            self.finish_external_formatted_save(result);
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
                .is_some_and(|pending| pending.is_lsp_request(event.request))
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
mod tests;
