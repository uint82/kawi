use super::*;
use crate::app::App;
use kawi_editor::Editor;
use std::path::PathBuf;
use std::time::Instant;

impl App {
    pub(in crate::app) fn open_document_symbols(&mut self) -> anyhow::Result<bool> {
        let Some(path) = path_absolute(&self.editor) else {
            self.show_message("E32: No file name".to_string());
            return Ok(false);
        };
        self.lsp.sync_editor(&self.editor);
        if !self.lsp.supports_document_symbols() {
            self.show_message("No language server supports document symbols".to_string());
            return Ok(false);
        }
        let version = self.editor.buffer_revision();
        let cursor = (self.editor.row(), self.editor.col());
        let root = self.picker_root();
        let wake = self.events.wake();
        let cached = self.symbol_cache.get(&path, version).map(<[_]>::to_vec);
        let stale = cached
            .is_none()
            .then(|| self.symbol_cache.latest(&path).map(<[_]>::to_vec))
            .flatten();
        let mut session = SymbolSession::document(root, wake);
        if let Some(symbols) = cached {
            session.set_document_symbols(
                symbols,
                cursor,
                Instant::now(),
                &self.editor,
                &mut self.lsp,
            );
        } else {
            if let Some(symbols) = stale {
                session.set_document_symbols(
                    symbols,
                    cursor,
                    Instant::now(),
                    &self.editor,
                    &mut self.lsp,
                );
            }
            match self.lsp.document_symbols(&self.editor, version) {
                Ok(request) => session.set_document_request(request),
                Err(message) => {
                    self.show_message(message);
                    if session.picker().rows().is_empty() {
                        return Ok(false);
                    }
                }
            }
        }
        self.symbol_picker = Some(session);
        Ok(false)
    }

    pub(in crate::app) fn open_workspace_symbols(&mut self) -> anyhow::Result<bool> {
        if !self.lsp.supports_workspace_symbols() {
            self.show_message("No language server supports workspace symbols".to_string());
            return Ok(false);
        }
        self.symbol_picker = Some(SymbolSession::workspace(
            self.picker_root(),
            self.events.wake(),
        ));
        Ok(false)
    }

    pub(in crate::app) fn handle_symbol_picker_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> anyhow::Result<bool> {
        let Some(mut session) = self.symbol_picker.take() else {
            return Ok(false);
        };
        let now = Instant::now();
        match session.handle_key(key, &self.editor, &mut self.lsp, now) {
            SymbolKeyOutcome::Consumed => self.symbol_picker = Some(session),
            SymbolKeyOutcome::Closed => {}
            SymbolKeyOutcome::Jump(jump) => {
                session.close(&mut self.lsp);
                self.apply_symbol_jump(jump);
            }
        }
        Ok(false)
    }

    pub(in crate::app) fn poll_symbol_events(&mut self) -> bool {
        let now = Instant::now();
        let mut changed = false;
        while let Some(event) = self.lsp.take_symbol_event() {
            changed = true;
            let jump = handle_event(
                event,
                &mut self.symbol_picker,
                &self.editor,
                &mut self.lsp,
                &mut self.symbol_cache,
                now,
            );
            if let Some(jump) = jump {
                if let Some(mut session) = self.symbol_picker.take() {
                    session.close(&mut self.lsp);
                }
                self.apply_symbol_jump(jump);
            }
        }
        changed
    }

    pub(in crate::app) fn symbols_tick(&mut self) -> bool {
        let now = Instant::now();
        self.symbol_picker
            .as_mut()
            .is_some_and(|session| session.tick(now, &mut self.lsp))
    }

    pub(in crate::app) fn symbols_wait_timeout(&self) -> Option<std::time::Duration> {
        self.symbol_picker
            .as_ref()
            .and_then(|session| session.wait_timeout(Instant::now()))
    }

    pub(in crate::app) fn symbol_picker_lines(&self) -> Vec<kawi_tui::frame::PickerLine<'_>> {
        let Some(session) = self.symbol_picker.as_ref() else {
            return Vec::new();
        };
        session
            .picker()
            .rows()
            .iter()
            .map(|row| kawi_tui::frame::PickerLine {
                primary: &row.display,
                detail: row.secondary.as_deref(),
                deprecated: row.deprecated,
            })
            .collect()
    }

    pub(in crate::app) fn symbol_preview(&self) -> Option<kawi_tui::frame::PickerPreview<'_>> {
        let session = self.symbol_picker.as_ref()?;
        crate::app::preview::preview_menu(session.preview_state(), self.editor.tabstop())
    }

    fn apply_symbol_jump(&mut self, jump: SymbolJump) {
        self.editor.record_jump();
        if let Err(message) =
            self.open_file_location(&jump.path, Some(jump.line), Some(jump.column))
        {
            self.show_message(message);
            return;
        }
        self.reveal_cursor();
    }
}

fn path_absolute(editor: &Editor) -> Option<PathBuf> {
    if !editor.has_file() {
        return None;
    }
    let path = PathBuf::from(editor.filename());
    Some(std::path::absolute(&path).unwrap_or(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::Flavor;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use kawi_buffer::Buffer;

    #[test]
    fn ctrl_o_returns_after_a_symbol_jump() {
        let root = std::env::temp_dir().join(format!("kawi_symbol_jump_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("project");
        let path = root.join("main.rs");
        std::fs::write(&path, "one\ntwo\nthree\n").expect("write");
        let mut app = App::new(
            Buffer::from_file(&path.to_string_lossy()).expect("buffer"),
            Flavor::Vim,
        );
        app.apply_symbol_jump(SymbolJump {
            path: path.clone(),
            line: 2,
            column: 1,
        });
        assert_eq!((app.editor.row(), app.editor.col()), (2, 1));
        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL))
            .expect("jump back");
        assert_eq!((app.editor.row(), app.editor.col()), (0, 0));
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
