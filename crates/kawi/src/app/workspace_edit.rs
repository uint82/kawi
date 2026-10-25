use std::fs;
use std::path::{Path, PathBuf};

use kawi_buffer::{apply_text_edits, content_hash, validate_text_edits, Buffer};
use kawi_lsp::{RenameEvent, TextChange, WorkspaceEdit};

use super::App;

struct PreparedDocument {
    path: PathBuf,
    edits: Vec<TextChange>,
    open: bool,
    original: Option<Vec<u8>>,
    updated: Option<String>,
}

impl App {
    pub(super) fn handle_rename_event(&mut self, event: RenameEvent) {
        match self.apply_workspace_edit(&event.edit) {
            Ok(count) => {
                self.show_message(format!(
                    "Renamed in {}",
                    kawi_tui::text::count_label(count, "file", "files")
                ));
                self.reveal_cursor();
            }
            Err(message) => self.show_message(message),
        }
    }

    pub(super) fn apply_workspace_edit(&mut self, edit: &WorkspaceEdit) -> Result<usize, String> {
        let prepared = self.prepare_documents(edit)?;
        self.write_closed_documents(&prepared)?;
        self.apply_open_documents(&prepared)?;
        self.exit_visual_if_active();
        Ok(prepared.len())
    }

    fn prepare_documents(&self, edit: &WorkspaceEdit) -> Result<Vec<PreparedDocument>, String> {
        edit.documents
            .iter()
            .map(|document| self.prepare_document(document))
            .collect()
    }

    fn prepare_document(
        &self,
        document: &kawi_lsp::DocumentEdit,
    ) -> Result<PreparedDocument, String> {
        let path = document.path.clone();
        if self.editor.file_is_open(&path) {
            return self.prepare_open_document(document);
        }
        self.prepare_closed_document(document)
    }

    fn prepare_open_document(
        &self,
        document: &kawi_lsp::DocumentEdit,
    ) -> Result<PreparedDocument, String> {
        let path = document.path.clone();
        if self.editor.file_is_modified(&path) && !self.editor.file_is_current(&path) {
            return Err(format!(
                "Save or revert modified buffer: {}",
                path.display()
            ));
        }
        if self.editor.file_hash(&path) != Some(document.original_hash) {
            return Err(format!("Buffer changed: {}", path.display()));
        }
        let length = self
            .editor
            .file_char_len(&path)
            .ok_or_else(|| format!("Buffer is not open: {}", path.display()))?;
        validate_text_edits(length, &document.edits).map_err(|_| invalid_edit(&path))?;
        Ok(PreparedDocument {
            path,
            edits: document.edits.clone(),
            open: true,
            original: None,
            updated: None,
        })
    }

    fn prepare_closed_document(
        &self,
        document: &kawi_lsp::DocumentEdit,
    ) -> Result<PreparedDocument, String> {
        let path = document.path.clone();
        let bytes =
            fs::read(&path).map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
        if content_hash(&bytes) != document.original_hash {
            return Err(format!("File changed on disk: {}", path.display()));
        }
        let text = String::from_utf8(bytes.clone())
            .map_err(|_| format!("Cannot edit non UTF-8 file: {}", path.display()))?;
        let updated = apply_text_edits(&Buffer::from_string(&text).snapshot(), &document.edits)
            .map_err(|_| invalid_edit(&path))?;
        Ok(PreparedDocument {
            path,
            edits: document.edits.clone(),
            open: false,
            original: Some(bytes),
            updated: Some(updated.to_string()),
        })
    }

    fn write_closed_documents(&self, prepared: &[PreparedDocument]) -> Result<(), String> {
        let mut written: Vec<&PreparedDocument> = Vec::new();
        for document in prepared.iter().filter(|document| !document.open) {
            let Some(updated) = document.updated.as_deref() else {
                continue;
            };
            if let Err(error) = write_file(&document.path, updated) {
                let message = format!("Cannot write {}: {error}", document.path.display());
                return Err(self.rollback(&written, message));
            }
            written.push(document);
        }
        Ok(())
    }

    fn apply_open_documents(&mut self, prepared: &[PreparedDocument]) -> Result<(), String> {
        for document in prepared.iter().filter(|document| document.open) {
            if let Err(error) = self
                .editor
                .apply_text_edits(&document.path, &document.edits)
            {
                return Err(self.rollback(
                    &prepared
                        .iter()
                        .filter(|item| !item.open)
                        .collect::<Vec<_>>(),
                    super::file_error_message(error),
                ));
            }
        }
        Ok(())
    }

    fn rollback(&self, written: &[&PreparedDocument], message: String) -> String {
        let mut failures = Vec::new();
        for document in written {
            let Some(original) = document.original.as_deref() else {
                continue;
            };
            if let Err(error) = write_bytes(&document.path, original) {
                failures.push(format!("{}: {error}", document.path.display()));
            }
        }
        if failures.is_empty() {
            message
        } else {
            format!("{message}; rollback failed for {}", failures.join(", "))
        }
    }
}

fn invalid_edit(path: &Path) -> String {
    format!("Invalid text edits for {}", path.display())
}

fn write_file(path: &Path, text: &str) -> std::io::Result<()> {
    write_bytes(path, text.as_bytes())
}

fn write_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    kawi_buffer::write::write(path, false, |out| out.write_all(bytes))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use kawi_buffer::Buffer;
    use kawi_editor::Action;
    use kawi_lsp::{DocumentEdit, RenameEvent, TextChange, WorkspaceEdit};

    use super::super::App;
    use super::*;
    use crate::keymap::Flavor;

    fn project(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("kawi_workspace_edit_{name}_{stamp}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("project");
        root
    }

    fn app_for(root: &Path, name: &str) -> App {
        App::new(
            Buffer::from_file(root.join(name).to_str().expect("path")).expect("buffer"),
            Flavor::Vim,
        )
    }

    fn document(path: &Path, text: &str, edits: Vec<TextChange>) -> DocumentEdit {
        DocumentEdit {
            path: path.to_path_buf(),
            original_hash: content_hash(text.as_bytes()),
            edits,
        }
    }

    fn rename_all(text: &str) -> Vec<TextChange> {
        let matches: Vec<TextChange> = text
            .match_indices("old")
            .map(|(start, value)| TextChange {
                start,
                end: start + value.len(),
                text: "new".to_string(),
            })
            .collect();
        matches
    }

    #[test]
    fn applies_edits_to_open_and_closed_files() {
        let root = project("mixed");
        let open = root.join("one.txt");
        let closed = root.join("two.txt");
        fs::write(&open, "old\n").expect("open");
        fs::write(&closed, "old old\n").expect("closed");
        let mut app = app_for(&root, "one.txt");
        let edit = WorkspaceEdit {
            documents: vec![
                document(&open, "old\n", rename_all("old\n")),
                document(&closed, "old old\n", rename_all("old old\n")),
            ],
        };
        assert_eq!(app.apply_workspace_edit(&edit), Ok(2));
        assert_eq!(app.editor.visible_lines(0, 1), vec!["new"]);
        assert_eq!(fs::read_to_string(&closed).expect("disk"), "new new\n");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn applies_a_rename_to_one_buffer_as_one_undo_step() {
        let root = project("undo_group");
        let path = root.join("a.txt");
        fs::write(&path, "old old\n").expect("file");
        let mut app = app_for(&root, "a.txt");
        let edit = WorkspaceEdit {
            documents: vec![document(&path, "old old\n", rename_all("old old\n"))],
        };
        assert_eq!(app.apply_workspace_edit(&edit), Ok(1));
        assert_eq!(app.editor.visible_lines(0, 1), vec!["new new"]);
        assert!(app.editor.try_undo());
        assert_eq!(app.editor.visible_lines(0, 1), vec!["old old"]);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn applies_edits_to_a_modified_current_buffer() {
        let root = project("dirty_current");
        let path = root.join("a.txt");
        fs::write(&path, "old\n").expect("file");
        let mut app = app_for(&root, "a.txt");
        app.editor.apply(Action::EnterInsert);
        app.editor.apply(Action::InsertChar('x'));
        app.editor.apply(Action::ExitInsert);
        let edit = WorkspaceEdit {
            documents: vec![document(
                &path,
                "xold\n",
                vec![TextChange {
                    start: 1,
                    end: 4,
                    text: "new".to_string(),
                }],
            )],
        };
        assert_eq!(app.apply_workspace_edit(&edit), Ok(1));
        assert_eq!(app.editor.visible_lines(0, 1), vec!["xnew"]);
        assert_eq!(fs::read_to_string(&path).expect("disk"), "old\n");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_stale_open_and_closed_files() {
        let root = project("stale");
        let open = root.join("one.txt");
        let closed = root.join("two.txt");
        fs::write(&open, "old\n").expect("open");
        fs::write(&closed, "old\n").expect("closed");
        let mut app = app_for(&root, "one.txt");
        let edit = WorkspaceEdit {
            documents: vec![document(&closed, "old\n", rename_all("old\n"))],
        };
        fs::write(&closed, "changed\n").expect("change");
        let error = app.apply_workspace_edit(&edit).expect_err("stale");
        assert!(error.contains("changed on disk"), "{error}");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn reports_rename_counts() {
        let root = project("event");
        let path = root.join("a.txt");
        fs::write(&path, "old\n").expect("file");
        let mut app = app_for(&root, "a.txt");
        app.handle_rename_event(RenameEvent {
            request: 7,
            edit: WorkspaceEdit {
                documents: vec![document(&path, "old\n", rename_all("old\n"))],
            },
        });
        assert_eq!(app.last_message(), Some("Renamed in 1 file"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn rolls_back_closed_files_when_a_later_write_fails() {
        use std::os::unix::fs::PermissionsExt;

        let root = project("rollback");
        let current = root.join("current.txt");
        let first = root.join("one.txt");
        let second = root.join("two.txt");
        fs::write(&current, "current\n").expect("current");
        fs::write(&first, "old\n").expect("first");
        fs::write(&second, "old\n").expect("second");
        fs::set_permissions(&second, fs::Permissions::from_mode(0o444)).expect("readonly");
        let mut app = app_for(&root, "current.txt");
        let edit = WorkspaceEdit {
            documents: vec![
                document(&first, "old\n", rename_all("old\n")),
                document(&second, "old\n", rename_all("old\n")),
            ],
        };
        let error = app.apply_workspace_edit(&edit).expect_err("readonly");
        assert!(error.contains("Cannot write"), "{error}");
        assert_eq!(fs::read_to_string(&first).expect("first"), "old\n");
        assert_eq!(fs::read_to_string(&second).expect("second"), "old\n");
        fs::set_permissions(&second, fs::Permissions::from_mode(0o644)).expect("writable");
        fs::remove_dir_all(root).expect("cleanup");
    }
}
