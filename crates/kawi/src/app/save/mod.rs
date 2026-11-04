use std::path::PathBuf;
use std::time::{Duration, Instant};

use kawi_lsp::FormattingEvent;

use super::App;
use external::ExternalSave;

const FORMAT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SaveAfter {
    None,
    Quit,
    QuitAll,
}

pub(super) enum SaveFormat {
    Lsp(u64),
    External {
        request: u64,
        path: PathBuf,
        hash: u64,
    },
}

pub(super) struct PendingSave {
    pub(super) format: SaveFormat,
    pub(super) number: usize,
    pub(super) force: bool,
    pub(super) after: SaveAfter,
    pub(super) visible: bool,
    pub(super) started: Instant,
}

impl PendingSave {
    pub(super) fn is_lsp_request(&self, request: u64) -> bool {
        matches!(self.format, SaveFormat::Lsp(current) if current == request)
    }

    pub(super) fn external_request(&self) -> Option<u64> {
        match self.format {
            SaveFormat::External { request, .. } => Some(request),
            SaveFormat::Lsp(_) => None,
        }
    }
}

mod external;

impl App {
    pub(super) fn request_save(&mut self, force: bool, after: SaveAfter, visible: bool) -> bool {
        if self.save_is_pending() {
            if visible {
                self.show_message("Save already in progress".to_string());
            }
            return false;
        }
        if !self.editor.has_file() {
            if visible {
                self.show_message("E32: No file name".to_string());
            }
            return false;
        }
        let number = self.editor.buffer_number();
        if self.directory.contains(number) {
            return self.request_directory_save(number, force, after, visible);
        }
        if let Some(message) = self.buffer_write_error(number) {
            if visible {
                self.show_message(message.to_string());
            }
            return false;
        }
        if self.disk_conflict(number) && !force {
            if visible {
                self.show_message(
                    "File changed on disk; resolve the conflict or use :w!".to_string(),
                );
            }
            return false;
        }
        if !self.format_on_save || !self.editor.is_dirty() {
            return self.save_buffer_now(number, force, after, visible, None);
        }
        if let Some((path, formatter)) = self.external_formatter() {
            return self.save_with_external_formatter(ExternalSave {
                number,
                path,
                formatter,
                force,
                after,
                visible,
            });
        }
        self.request_lsp_formatted_save(number, force, after, visible)
    }

    fn request_lsp_formatted_save(
        &mut self,
        number: usize,
        force: bool,
        after: SaveAfter,
        visible: bool,
    ) -> bool {
        self.lsp.refresh(&self.editor);
        match self.lsp.formatting(&self.editor, None) {
            Ok(request) => {
                self.pending_save = Some(PendingSave {
                    format: SaveFormat::Lsp(request),
                    number,
                    force,
                    after,
                    visible,
                    started: Instant::now(),
                });
                false
            }
            Err(_) => self.save_buffer_now(number, force, after, visible, None),
        }
    }

    pub(super) fn save_tick(&mut self) -> bool {
        let Some(pending) = self.pending_save.as_ref() else {
            return false;
        };
        if pending.started.elapsed() < FORMAT_TIMEOUT {
            return false;
        }
        let Some(pending) = self.pending_save.take() else {
            return false;
        };
        if matches!(pending.format, SaveFormat::Lsp(_)) {
            self.lsp.cancel_formatting();
        }
        self.save_buffer_now(
            pending.number,
            pending.force,
            pending.after,
            pending.visible,
            Some("formatter timed out; saved without formatting"),
        );
        true
    }

    pub(super) fn save_wait_timeout(&self) -> Option<Duration> {
        self.pending_save
            .as_ref()
            .map(|pending| FORMAT_TIMEOUT.saturating_sub(pending.started.elapsed()))
    }

    pub(super) fn finish_formatted_save(&mut self, event: FormattingEvent) {
        let Some(pending) = self.pending_save.take() else {
            return;
        };
        if !pending.is_lsp_request(event.request) {
            self.pending_save = Some(pending);
            return;
        }
        if self.disk_conflict(pending.number) && !pending.force {
            if pending.visible {
                self.show_message("File changed on disk; save cancelled".to_string());
            }
            return;
        }
        match self.apply_workspace_edit(&event.edit) {
            Ok(_) => {
                self.save_buffer_now(
                    pending.number,
                    pending.force,
                    pending.after,
                    pending.visible,
                    None,
                );
            }
            Err(message) => {
                let suffix = format!(
                    "{}; saved without formatting",
                    super::lsp::one_line(&message)
                );
                self.save_buffer_now(
                    pending.number,
                    pending.force,
                    pending.after,
                    pending.visible,
                    Some(&suffix),
                );
            }
        }
    }

    pub(super) fn finish_formatting_failure(&mut self, request: u64, message: String) {
        let Some(pending) = self.pending_save.take() else {
            return;
        };
        if !pending.is_lsp_request(request) {
            self.pending_save = Some(pending);
            return;
        }
        let suffix = (message != "No formatting changes").then(|| {
            format!(
                "{}; saved without formatting",
                super::lsp::one_line(&message)
            )
        });
        self.save_buffer_now(
            pending.number,
            pending.force,
            pending.after,
            pending.visible,
            suffix.as_deref(),
        );
    }

    fn buffer_write_error(&self, number: usize) -> Option<&'static str> {
        if self.editor.buffer_is_internal(number).unwrap_or(false) {
            return Some("E382: Cannot write, internal buffer");
        }
        self.editor
            .buffer_is_readonly(number)
            .unwrap_or(false)
            .then_some("E45: 'readonly' option is set")
    }

    fn save_buffer_now(
        &mut self,
        number: usize,
        force: bool,
        after: SaveAfter,
        visible: bool,
        suffix: Option<&str>,
    ) -> bool {
        if let Some(message) = self.buffer_write_error(number) {
            if visible {
                self.show_message(message.to_string());
            }
            return false;
        }
        if self.disk_conflict(number) && !force {
            if visible {
                self.show_message(
                    "File changed on disk; resolve the conflict or use :w!".to_string(),
                );
            }
            return false;
        }
        let name = self
            .editor
            .buffer_name(number)
            .unwrap_or("[No Name]")
            .to_string();
        let path = self.editor.buffer_path(number).map(PathBuf::from);
        let current = number == self.editor.buffer_number();
        if current {
            self.lsp.refresh(&self.editor);
        }
        match self.editor.save_buffer(number, force) {
            Ok(outcome) => {
                let report = super::written_report(&name, outcome);
                self.persist_undo(number);
                self.record_saved_buffer_number(number);
                self.plugins.file_saved(
                    number,
                    self.editor.buffer_path(number).map(str::to_string),
                    self.editor.buffer_revision_at(number).unwrap_or_default(),
                );
                if number == self.editor.buffer_number() {
                    if let Some(path) = path {
                        self.lsp
                            .save_path(&std::path::absolute(&path).unwrap_or(path));
                    }
                }
                if visible {
                    match suffix {
                        Some(suffix) => self.show_message(format!("{report}; {suffix}")),
                        None => self.show_message(report),
                    }
                } else if let Some(suffix) = suffix {
                    self.show_message(format!("Autosave: {suffix}"));
                }
                self.complete_save(after);
                true
            }
            Err(error) => {
                self.report_error(super::file_error_message(error));
                false
            }
        }
    }

    pub(super) fn complete_save(&mut self, after: SaveAfter) {
        if after != SaveAfter::Quit {
            return;
        }
        if self.handle_quit().unwrap_or(false) {
            self.quit_pending = true;
        }
    }
}

#[cfg(test)]
mod tests;
