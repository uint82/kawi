use std::path::PathBuf;
use std::time::Instant;

use kawi_lsp::FormatterConfig;

use super::super::formatter::FormatError;
use super::super::formatter_service::FormatResult;
use super::super::App;
use super::{PendingSave, SaveAfter, SaveFormat};

pub(super) struct ExternalSave {
    pub(super) number: usize,
    pub(super) path: PathBuf,
    pub(super) formatter: FormatterConfig,
    pub(super) force: bool,
    pub(super) after: SaveAfter,
    pub(super) visible: bool,
}

impl App {
    pub(super) fn save_with_external_formatter(&mut self, save: ExternalSave) -> bool {
        let ExternalSave {
            number,
            path,
            formatter,
            force,
            after,
            visible,
        } = save;
        let Some(text) = self.editor.buffer_text(number) else {
            return self.save_buffer_now(number, force, after, visible, None);
        };
        let Some(hash) = self.editor.file_hash(&path) else {
            return self.save_buffer_now(number, force, after, visible, None);
        };
        match self.queue_formatter_request(path.clone(), formatter, text) {
            Ok(request) => {
                self.pending_save = Some(PendingSave {
                    format: SaveFormat::External {
                        request,
                        path,
                        hash,
                    },
                    number,
                    force,
                    after,
                    visible,
                    started: Instant::now(),
                });
                false
            }
            Err(message) => {
                if visible {
                    self.show_message(format!("{message}; using language server"));
                }
                self.request_lsp_formatted_save(number, force, after, visible)
            }
        }
    }

    pub(in crate::app) fn finish_external_formatted_save(&mut self, result: FormatResult) {
        let Some(pending) = self.pending_save.take() else {
            return;
        };
        if pending.external_request() != Some(result.request) {
            self.pending_save = Some(pending);
            return;
        }
        match result.result {
            Ok(formatted) => {
                self.apply_external_save(pending, formatted);
            }
            Err(FormatError::NotFound(message)) => {
                if pending.visible {
                    self.show_message(format!("{message}; using language server"));
                }
                self.request_lsp_formatted_save(
                    pending.number,
                    pending.force,
                    pending.after,
                    pending.visible,
                );
            }
            Err(FormatError::Failed(message)) => {
                let suffix = format!(
                    "{}; saved without formatting",
                    super::super::lsp::one_line(&message)
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

    fn apply_external_save(&mut self, pending: PendingSave, formatted: String) -> bool {
        let SaveFormat::External { path, hash, .. } = &pending.format else {
            return false;
        };
        let path = path.clone();
        let hash = *hash;
        if self.editor.file_hash(&path) != Some(hash) {
            return self.save_buffer_now(
                pending.number,
                pending.force,
                pending.after,
                pending.visible,
                Some("buffer changed; saved without formatting"),
            );
        }
        let Some(current) = self.editor.buffer_text(pending.number) else {
            return self.save_buffer_now(
                pending.number,
                pending.force,
                pending.after,
                pending.visible,
                Some("buffer unavailable; saved without formatting"),
            );
        };
        if current != formatted {
            if let Err(message) = self.apply_formatted_text(&path, &formatted) {
                let suffix = format!(
                    "{}; saved without formatting",
                    super::super::lsp::one_line(&message)
                );
                return self.save_buffer_now(
                    pending.number,
                    pending.force,
                    pending.after,
                    pending.visible,
                    Some(&suffix),
                );
            }
        }
        self.save_buffer_now(
            pending.number,
            pending.force,
            pending.after,
            pending.visible,
            None,
        )
    }
}
