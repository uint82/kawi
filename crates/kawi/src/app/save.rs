use std::path::PathBuf;
use std::time::{Duration, Instant};

use kawi_lsp::FormattingEvent;

use super::App;

const FORMAT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SaveAfter {
    None,
    Quit,
}

pub(super) struct PendingSave {
    request: u64,
    number: usize,
    force: bool,
    after: SaveAfter,
    visible: bool,
    started: Instant,
}

impl PendingSave {
    pub(super) fn matches(&self, request: u64) -> bool {
        self.request == request
    }
}

impl App {
    pub(super) fn request_save(&mut self, force: bool, after: SaveAfter, visible: bool) -> bool {
        if self.pending_save.is_some() {
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
        self.lsp.refresh(&self.editor);
        match self.lsp.formatting(&self.editor, None) {
            Ok(request) => {
                self.pending_save = Some(PendingSave {
                    request,
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
        self.lsp.cancel_formatting();
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
        if pending.request != event.request {
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
        if pending.request != request {
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

    fn save_buffer_now(
        &mut self,
        number: usize,
        force: bool,
        after: SaveAfter,
        visible: bool,
        suffix: Option<&str>,
    ) -> bool {
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
                self.record_saved_buffer_number(number);
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

    fn complete_save(&mut self, after: SaveAfter) {
        if after != SaveAfter::Quit {
            return;
        }
        if self.handle_quit().unwrap_or(false) {
            self.quit_pending = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use kawi_buffer::{content_hash, Buffer};
    use kawi_editor::{Action, WriteOutcome};
    use kawi_lsp::{DocumentEdit, FormattingEvent, TextChange, WorkspaceEdit};

    use super::*;
    use crate::keymap::Flavor;

    fn app_with(name: &str, text: &str) -> (PathBuf, PathBuf, App) {
        let dir = crate::test_support::temp_dir(name);
        let path = dir.join("a.txt");
        fs::write(&path, text).expect("write");
        let app = App::new(
            Buffer::from_file(path.to_str().expect("path")).expect("buffer"),
            Flavor::Vim,
        );
        (dir, path, app)
    }

    fn message(app: &App) -> String {
        app.last_message()
            .map(str::to_string)
            .or_else(|| app.pager_lines().map(|lines| lines.join(" ")))
            .unwrap_or_default()
    }

    fn edit(app: &mut App, text: &str) {
        app.editor.apply(Action::EnterInsert);
        for ch in text.chars() {
            app.editor.apply(Action::InsertChar(ch));
        }
        app.editor.apply(Action::ExitInsert);
    }

    fn pending(app: &mut App, request: u64, path: &Path) {
        app.pending_save = Some(PendingSave {
            request,
            number: 1,
            force: false,
            after: SaveAfter::None,
            visible: true,
            started: Instant::now(),
        });
        assert!(path.exists());
    }

    fn formatting_event(request: u64, path: &Path, original: &str) -> FormattingEvent {
        FormattingEvent {
            request,
            edit: WorkspaceEdit {
                documents: vec![DocumentEdit {
                    path: path.to_path_buf(),
                    original_hash: content_hash(original.as_bytes()),
                    edits: vec![TextChange {
                        start: 0,
                        end: original.len(),
                        text: "formatted\n".to_string(),
                    }],
                }],
            },
        }
    }

    #[test]
    fn saves_directly_when_formatting_is_disabled() {
        let (dir, path, mut app) = app_with("direct", "alpha\n");
        edit(&mut app, "x");
        app.request_save(false, SaveAfter::None, true);
        assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
        assert_eq!(
            app.editor.buffer_text(1).as_deref(),
            Some("xalpha\n"),
            "buffer text"
        );
        assert_eq!(
            app.editor.buffer_is_dirty(1),
            Some(false),
            "buffer should be clean"
        );
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn missing_formatter_falls_back_to_a_direct_save() {
        let (dir, path, mut app) = app_with("no_formatter", "alpha\n");
        app.format_on_save = true;
        edit(&mut app, "x");
        app.request_save(false, SaveAfter::None, true);
        assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
        assert!(app.pending_save.is_none());
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn formatted_save_applies_edits_before_writing() {
        let (dir, path, mut app) = app_with("formatted", "alpha\n");
        edit(&mut app, "x");
        pending(&mut app, 7, &path);
        app.finish_formatted_save(formatting_event(7, &path, "xalpha\n"));
        assert_eq!(fs::read_to_string(&path).expect("disk"), "formatted\n");
        assert!(!fs::read_to_string(&path).expect("disk").contains("xalpha"));
        assert_eq!(app.editor.buffer_text(1).as_deref(), Some("formatted\n"));
        assert_eq!(app.editor.buffer_is_dirty(1), Some(false));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn formatting_is_one_undoable_step() {
        let (dir, path, mut app) = app_with("undo", "alpha\n");
        edit(&mut app, "x");
        pending(&mut app, 12, &path);
        app.finish_formatted_save(formatting_event(12, &path, "xalpha\n"));
        assert!(app.editor.try_undo());
        assert_eq!(app.editor.buffer_text(1).as_deref(), Some("xalpha\n"));
        assert!(app.editor.is_dirty());
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn write_quit_requests_quit_after_saving() {
        let (dir, path, mut app) = app_with("wq", "alpha\n");
        edit(&mut app, "x");
        app.handle_write_quit(false).expect("write quit");
        assert!(app.quit_pending);
        assert_eq!(fs::read_to_string(path).expect("disk"), "xalpha\n");
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn stale_formatting_saves_the_current_buffer_unformatted() {
        let (dir, path, mut app) = app_with("stale", "alpha\n");
        edit(&mut app, "x");
        pending(&mut app, 8, &path);
        app.finish_formatted_save(formatting_event(8, &path, "different\n"));
        assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
        assert!(message(&app).contains("saved without formatting"));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn formatter_timeout_saves_unformatted() {
        let (dir, path, mut app) = app_with("timeout", "alpha\n");
        edit(&mut app, "x");
        pending(&mut app, 9, &path);
        app.pending_save.as_mut().expect("pending").started = Instant::now() - FORMAT_TIMEOUT;
        assert!(app.save_tick());
        assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
        assert!(message(&app).contains("formatter timed out"));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn formatter_failure_saves_unformatted() {
        let (dir, path, mut app) = app_with("failure", "alpha\n");
        edit(&mut app, "x");
        pending(&mut app, 10, &path);
        app.finish_formatting_failure(10, "server crashed".to_string());
        assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
        assert!(message(&app).contains("server crashed"));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn no_formatting_changes_saves_normally() {
        let (dir, path, mut app) = app_with("no_changes", "alpha\n");
        edit(&mut app, "x");
        pending(&mut app, 11, &path);
        app.finish_formatting_failure(11, "No formatting changes".to_string());
        assert_eq!(fs::read_to_string(&path).expect("disk"), "xalpha\n");
        assert!(!message(&app).contains("without formatting"));
        fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn report_shape_matches_editor_output() {
        assert_eq!(
            super::super::written_report(
                "notes.txt",
                WriteOutcome {
                    lines: 2,
                    bytes: 4,
                    created: false,
                },
            ),
            "\"notes.txt\" 2L, 4B written"
        );
    }
}
