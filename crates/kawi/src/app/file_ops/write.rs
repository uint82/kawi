use std::path::{Path, PathBuf};

use kawi_editor::paths;

use super::prompt::{ConfirmSession, FileOpSession, PendingWrite, WriteKind};
use super::{absolute_path, fs, is_directory_text, App};

impl App {
    pub(super) fn handle_write_target(&mut self, name: &str, force: bool, kind: WriteKind) {
        self.prepare_write_target(absolute_path(name), name.to_string(), force, kind);
    }

    pub(super) fn create_parents_and_write(&mut self, pending: PendingWrite) {
        let target = absolute_path(&pending.name);
        let Some(parent) = fs::missing_parent(&target) else {
            self.perform_write(pending);
            return;
        };
        if let Err(error) = fs::create_directory(&parent) {
            self.show_message(format!("Cannot create {}: {error}", parent.display()));
            return;
        }
        self.perform_write(pending);
    }

    fn prepare_write_target(
        &mut self,
        target: PathBuf,
        name: String,
        force: bool,
        kind: WriteKind,
    ) {
        let pending = PendingWrite { name, force, kind };
        match fs::missing_parent(&target) {
            Some(_) if force => self.create_parents_and_write(pending),
            Some(parent) => {
                let label = format!("Create directory \"{}\"? [y/N]", parent.display());
                self.file_ops = Some(FileOpSession::Confirm(ConfirmSession::create_parents(
                    label, pending,
                )));
            }
            None => self.perform_write(pending),
        }
    }

    fn perform_write(&mut self, pending: PendingWrite) {
        match pending.kind {
            WriteKind::SaveAs => self.write_save_as(&pending.name, pending.force),
            WriteKind::WriteAs => {
                self.write_as_and_report(&pending.name, pending.force);
            }
            WriteKind::CreateFile => self.create_file(&pending.name),
            WriteKind::CreateDirectory => self.create_directory(&pending.name),
        }
    }

    fn write_save_as(&mut self, name: &str, force: bool) {
        let number = self.editor.buffer_number();
        match self.editor.save_as(name, force) {
            Ok(outcome) => {
                let report = super::super::written_report(name, outcome);
                self.persist_undo(number);
                self.record_saved_buffer_number(number);
                self.lsp.refresh(&self.editor);
                self.lsp.save();
                self.sync_language_settings();
                self.sync_watched_files();
                self.refresh_git_status();
                self.show_message(report);
            }
            Err(error) => self.show_message(super::super::file_error_message(error)),
        }
    }

    pub(super) fn create_new_path(&mut self, root: &Path, text: &str) {
        let target = paths::absolute(text, root);
        if is_directory_text(text) {
            self.prepare_write_target(
                target.clone(),
                target.to_string_lossy().into_owned(),
                false,
                WriteKind::CreateDirectory,
            );
            return;
        }
        if fs::path_exists(&target) {
            self.show_message("E13: File exists (add ! to override)".to_string());
            return;
        }
        self.prepare_write_target(
            target.clone(),
            target.to_string_lossy().into_owned(),
            false,
            WriteKind::CreateFile,
        );
    }

    fn create_file(&mut self, name: &str) {
        let target = PathBuf::from(name);
        if let Err(error) = fs::create_empty_file(&target) {
            self.show_message(format!("Cannot create {}: {error}", target.display()));
            return;
        }
        self.open_created_path(&target);
    }

    fn create_directory(&mut self, name: &str) {
        let target = PathBuf::from(name);
        match fs::create_directory(&target) {
            Ok(()) => {
                self.refresh_git_status();
                self.show_message(format!("Created directory \"{}\"", target.display()));
            }
            Err(error) => {
                self.show_message(format!("Cannot create {}: {error}", target.display()));
            }
        }
    }

    fn open_created_path(&mut self, target: &Path) {
        let name = target.to_string_lossy();
        if let Err(error) = self.editor.open_file_buffer(&name) {
            self.show_message(super::super::file_error_message(error));
            return;
        }
        let number = self.editor.buffer_number();
        self.record_saved_buffer_number(number);
        self.lsp.refresh(&self.editor);
        self.sync_language_settings();
        self.sync_watched_files();
        self.refresh_git_status();
        self.show_message(format!("Created \"{}\"", target.display()));
    }
}
