use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::LspUi;

const CHANGE_DEBOUNCE: Duration = Duration::from_millis(150);

pub(super) struct PendingChange {
    path: PathBuf,
    text: ropey::Rope,
    due: Instant,
}

impl LspUi {
    pub(super) fn queue_change(&mut self, path: PathBuf, text: ropey::Rope) {
        let due = Instant::now() + CHANGE_DEBOUNCE;
        if self
            .pending_change
            .as_ref()
            .is_some_and(|pending| pending.path != path)
        {
            self.flush_pending_change();
        }
        match self.pending_change.as_mut() {
            Some(pending) => {
                pending.text = text;
                pending.due = due;
            }
            None => {
                self.pending_change = Some(PendingChange { path, text, due });
            }
        }
    }

    pub(super) fn flush_pending_change_if_due(&mut self, now: Instant) {
        if self
            .pending_change
            .as_ref()
            .is_some_and(|pending| pending.due <= now)
        {
            self.flush_pending_change();
        }
    }

    pub(super) fn flush_pending_change(&mut self) {
        let Some(pending) = self.pending_change.take() else {
            return;
        };
        let Some(service) = self.service.as_ref() else {
            return;
        };
        if service.change(&pending.path, pending.text) {
            self.diagnostics.advance();
        }
    }

    pub(in crate::app) fn change_wait_timeout(&self, now: Instant) -> Option<Duration> {
        self.pending_change
            .as_ref()
            .map(|pending| pending.due.saturating_duration_since(now))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use kawi_editor::Editor;

    use crate::app::lsp::LspUi;

    #[test]
    fn document_changes_are_coalesced_until_flushed() {
        let root = std::env::temp_dir().join(format!("kawi_lsp_debounce_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("main.rs");
        let mut buffer = kawi_buffer::Buffer::from_string("one\n");
        buffer.path = Some(path.to_string_lossy().into_owned());
        let mut editor = Editor::new(buffer);
        let mut ui = LspUi::default();
        ui.sync_document(&editor);
        editor.apply(kawi_editor::Action::InsertChar('x'));
        ui.sync_document(&editor);
        assert!(ui.pending_change.is_some());
        assert!(ui.change_wait_timeout(Instant::now()).is_some());
        editor.apply(kawi_editor::Action::InsertChar('y'));
        ui.sync_document(&editor);
        assert_eq!(
            ui.pending_change
                .as_ref()
                .map(|pending| pending.text.to_string())
                .as_deref(),
            Some("xyone\n")
        );
        ui.flush_pending_change();
        assert!(ui.pending_change.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn changing_documents_discards_the_previous_pending_change() {
        let mut ui = LspUi::default();
        let mut first = kawi_buffer::Buffer::from_string("one\n");
        first.path = Some("one.rs".to_string());
        let mut first = Editor::new(first);
        ui.sync_document(&first);
        first.apply(kawi_editor::Action::InsertChar('x'));
        ui.sync_document(&first);
        assert!(ui.pending_change.is_some());

        let mut second = kawi_buffer::Buffer::from_string("two\n");
        second.path = Some("two.rs".to_string());
        let second = Editor::new(second);
        ui.sync_document(&second);
        assert!(ui.pending_change.is_none());
    }
}
