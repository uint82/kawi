use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kawi_editor::paths;

use super::window::Windows;
use super::App;
use crate::session::{Session, SessionBuffer, SessionWindow, SESSION_VERSION};

mod pending;
pub(crate) use pending::{PendingBufferLoad, PendingSessionRestore};

impl App {
    pub(crate) fn session_snapshot(&mut self) -> Option<Session> {
        self.sync_active_window();
        let cwd = std::fs::canonicalize(std::env::current_dir().ok()?).ok()?;
        let mut indices = BTreeMap::new();
        let mut buffers = Vec::new();
        for number in self.editor.buffer_numbers() {
            let path = self
                .editor
                .buffer_path(number)
                .map(|path| paths::absolute(path, &cwd));
            indices.insert(number, buffers.len());
            buffers.push(SessionBuffer {
                path: path.map(path_text),
                id: self.editor.buffer_id(number).map(str::to_string),
            });
        }
        let windows = self.windows.snapshot(&indices)?;
        Some(Session {
            version: SESSION_VERSION,
            cwd: path_text(cwd),
            buffers,
            windows,
            active_window: self.windows.active(),
            explorer: self.explorer.is_enabled().then(|| self.explorer.snapshot()),
        })
    }

    pub(crate) fn restore_session(&mut self, session: &Session) -> Option<String> {
        let mut warnings = Vec::new();
        let visible = visible_buffer_indices(&session.windows);
        let (buffers, pending) =
            self.load_session_buffers(&session.buffers, &visible, &mut warnings);
        self.pending_session_buffers = pending;
        let Some(mut windows) =
            Windows::from_session(&session.windows, session.active_window, &buffers)
        else {
            warnings.push("session: no windows could be restored".to_string());
            self.reset_windows();
            return session_warnings(warnings);
        };
        windows.clamp_views(|buffer, view| {
            if self.directory_loading.is_loading(buffer) {
                view
            } else {
                self.editor.clamp_view_state(buffer, view)
            }
        });
        let buffer = windows.active_view().buffer;
        let view = windows.active_view().view;
        if self.editor.buffer_number() != buffer && !self.editor.buffer_select(buffer) {
            warnings.push("session: active buffer could not be restored".to_string());
            self.reset_windows();
            return session_warnings(warnings);
        }
        self.windows = windows;
        self.windows.set_all_options(self.window_defaults);
        self.window_syntax.clear();
        self.window_prefix = false;
        self.explorer
            .restore(Path::new(&session.cwd), session.explorer.as_ref());
        self.refresh_window_sizes();
        self.editor.restore_view_state(view);
        self.editor.set_window_options(self.window_defaults);
        self.reveal_cursor();
        self.sync_active_window();
        self.sync_language_settings();
        session_warnings(warnings)
    }

    fn load_session_buffers(
        &mut self,
        buffers: &[SessionBuffer],
        visible: &BTreeSet<usize>,
        warnings: &mut Vec<String>,
    ) -> (Vec<Option<usize>>, Vec<PendingBufferLoad>) {
        let mut numbers = Vec::with_capacity(buffers.len());
        let mut pending = Vec::new();
        let mut pending_numbers = BTreeSet::new();
        let mut used_initial = false;
        let mut skipped = 0;
        for (index, buffer) in buffers.iter().enumerate() {
            match buffer.path.as_deref() {
                Some(path) => {
                    let target = Path::new(path);
                    if !target.is_file() && !target.is_dir() {
                        skipped += 1;
                        numbers.push(None);
                        continue;
                    }
                    let eager = !used_initial || target.is_dir() || visible.contains(&index);
                    if !eager {
                        let number = self.editor.add_file_placeholder(path);
                        restore_buffer_id(&mut self.editor, number, buffer.id.as_deref());
                        if self.editor.is_buffer_pending(number) && pending_numbers.insert(number) {
                            pending.push(PendingBufferLoad {
                                number,
                                path: path.to_string(),
                            });
                        }
                        numbers.push(Some(number));
                        used_initial = true;
                        continue;
                    }
                    let opened = if target.is_dir() {
                        self.open_directory(target, !used_initial)
                            .map(|_| ())
                            .map_err(|_| ())
                    } else if used_initial {
                        self.editor.open_file_buffer(path).map_err(|_| ())
                    } else {
                        self.editor.edit_file(Some(path), false).map_err(|_| ())
                    };
                    match opened {
                        Ok(()) => {
                            let number = self.editor.buffer_number();
                            restore_buffer_id(&mut self.editor, number, buffer.id.as_deref());
                            numbers.push(Some(number));
                            used_initial = true;
                        }
                        Err(_) => {
                            skipped += 1;
                            numbers.push(None);
                        }
                    }
                }
                None => {
                    let number = if used_initial {
                        self.editor.open_empty_buffer();
                        self.editor.buffer_number()
                    } else {
                        self.editor.buffer_number()
                    };
                    restore_buffer_id(&mut self.editor, number, buffer.id.as_deref());
                    numbers.push(Some(number));
                    used_initial = true;
                }
            }
        }
        if skipped > 0 {
            warnings.push(skipped_buffers_warning(skipped));
        }
        (numbers, pending)
    }

    fn reset_windows(&mut self) {
        let options = self.editor.window_options();
        self.windows = Windows::new(self.editor.buffer_number(), self.editor.view_state());
        self.windows.set_all_options(options);
        self.window_syntax.clear();
        self.window_prefix = false;
        self.refresh_window_sizes();
        self.reveal_cursor();
    }
}

fn visible_buffer_indices(window: &SessionWindow) -> BTreeSet<usize> {
    let mut indices = BTreeSet::new();
    collect_window_buffers(window, &mut indices);
    indices
}

fn collect_window_buffers(window: &SessionWindow, indices: &mut BTreeSet<usize>) {
    match window {
        SessionWindow::Leaf { buffer, .. } => {
            indices.insert(*buffer);
        }
        SessionWindow::Split { first, second, .. } => {
            collect_window_buffers(first, indices);
            collect_window_buffers(second, indices);
        }
    }
}

fn restore_buffer_id(editor: &mut kawi_editor::Editor, number: usize, id: Option<&str>) {
    if let Some(id) = id {
        editor.set_buffer_id(number, id.to_string());
    }
}

fn path_text(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn skipped_buffers_warning(count: usize) -> String {
    let noun = if count == 1 { "buffer" } else { "buffers" };
    format!("session: skipped {count} unavailable {noun}")
}

fn session_warnings(warnings: Vec<String>) -> Option<String> {
    (!warnings.is_empty()).then(|| warnings.join("\n"))
}

#[cfg(test)]
mod tests;
