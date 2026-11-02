use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::pending::{PendingFileOp, RenamePending};
use super::{absolute_path, with_warning, App};
use crate::app::apply_journal;
use crate::app::directory::executor::PreparedPlan;
use crate::app::directory::fs::RealFs;
use crate::app::directory::planner::{Operation, Plan};
use crate::recovery::JournalCleanup;

impl App {
    pub(super) fn rename_file(&mut self, source: &Path, target: &str, force: bool) {
        let old = absolute_path(&source.to_string_lossy());
        let new = absolute_path(target);
        if old == new {
            self.show_message("Old and new path are the same".to_string());
            return;
        }
        let metadata = match fs::symlink_metadata(&old) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.show_message(format!("E484: Can't open file {}: {error}", old.display()));
                return;
            }
        };
        if let Some(parent) = super::fs::missing_parent(&new) {
            self.show_message(format!(
                "E484: Can't open file {}: parent directory {} does not exist",
                new.display(),
                parent.display()
            ));
            return;
        }
        let directory = metadata.is_dir() && !metadata.file_type().is_symlink();
        let request = if self.lsp.supports_file_rename() {
            self.lsp.will_rename_files(&[(old.clone(), new.clone())])
        } else {
            0
        };
        if request != 0 {
            self.pending_file_op = Some(PendingFileOp::Rename(RenamePending {
                request,
                source: old.clone(),
                target: new,
                force,
                directory,
            }));
            self.show_message(format!("Renaming \"{}\"...", old.display()));
            return;
        }
        self.finish_rename(old, new, force, directory);
    }

    pub(super) fn finish_rename(
        &mut self,
        old: PathBuf,
        new: PathBuf,
        force: bool,
        directory: bool,
    ) {
        if let Err(error) = super::fs::check_rename(&old, &new, force) {
            self.show_rename_error(&old, &new, error);
            return;
        }
        let symlink =
            fs::symlink_metadata(&old).is_ok_and(|metadata| metadata.file_type().is_symlink());
        let applied = match self.execute_rename(&old, &new, force) {
            Ok(applied) => applied,
            Err(error) => {
                self.show_message(error);
                return;
            }
        };
        self.after_rename(&old, &new);
        let message = rename_message(&old, &new, symlink, applied.copied, directory);
        self.show_message(with_warning(message, applied.warning));
    }

    fn execute_rename(&self, old: &Path, new: &Path, force: bool) -> Result<RenameApply, String> {
        let plan = rename_plan(old.to_path_buf(), new.to_path_buf(), force);
        let mut fs = RealFs::new(false);
        let prepared = PreparedPlan::new(&plan, &fs)?;
        let journal = self.create_rename_journal(&prepared)?;
        let written = match journal.as_ref() {
            Some(journal) => apply_journal::persist(self.recovery.as_ref(), journal)?,
            None => false,
        };
        let promotions = self.recovery.as_ref().map(|manager| manager.store_clone());
        let mut record_promotion = |operation| match &promotions {
            Some(store) => store
                .append_apply_promotion(operation)
                .map_err(io::Error::other),
            None => Ok(()),
        };
        run_rename_plan(
            self.recovery.as_ref(),
            journal.as_ref(),
            written,
            prepared,
            &mut fs,
            &mut record_promotion,
        )
    }

    fn after_rename(&mut self, old: &Path, new: &Path) {
        self.lsp
            .did_rename_files(&[(old.to_path_buf(), new.to_path_buf())]);
        self.rewrite_paths(old, new);
        self.lsp.refresh(&self.editor);
        self.sync_language_settings();
        self.sync_watched_files();
        self.refresh_git_status();
    }

    fn create_rename_journal(
        &self,
        prepared: &PreparedPlan<'_>,
    ) -> Result<Option<crate::recovery::ApplyJournal>, String> {
        if self.recovery.is_none() {
            return Ok(None);
        }
        apply_journal::create_for_plan(prepared).map(Some)
    }

    fn show_rename_error(&mut self, old: &Path, new: &Path, error: io::Error) {
        if error.kind() == io::ErrorKind::AlreadyExists {
            self.show_message("E13: File exists (add ! to override)".to_string());
        } else {
            self.show_message(format!(
                "Cannot rename {} to {}: {error}",
                old.display(),
                new.display()
            ));
        }
    }
}

struct RenameApply {
    copied: bool,
    warning: Option<String>,
}

fn run_rename_plan(
    manager: Option<&crate::recovery::RecoveryManager>,
    journal: Option<&crate::recovery::ApplyJournal>,
    written: bool,
    prepared: PreparedPlan<'_>,
    fs: &mut RealFs,
    record_promotion: &mut dyn FnMut(usize) -> io::Result<()>,
) -> Result<RenameApply, String> {
    let outcome = prepared.execute(fs, record_promotion);
    if let Some(error) = outcome.error.as_ref() {
        let warning =
            apply_journal::finish_failure(manager, outcome.rollback_errors.is_empty(), written);
        return Err(with_warning(error.clone(), warning));
    }
    let cleanups = outcome
        .deferred_deletes
        .iter()
        .cloned()
        .map(JournalCleanup::remove)
        .collect::<Vec<_>>();
    let warning = apply_journal::finish_success(manager, journal, &cleanups, written);
    Ok(RenameApply {
        copied: !outcome.deferred_deletes.is_empty(),
        warning,
    })
}

fn rename_message(old: &Path, new: &Path, symlink: bool, copied: bool, directory: bool) -> String {
    let mut note = String::new();
    if symlink {
        note.push_str(" (symlink moved, target unchanged)");
    }
    if copied {
        note.push_str(" (copied and deleted source across filesystems)");
    }
    let kind = if directory { "directory" } else { "file" };
    format!(
        "Renamed {kind} \"{}\" to \"{}\"{note}",
        old.display(),
        new.display()
    )
}

fn rename_plan(from: PathBuf, to: PathBuf, force: bool) -> Plan {
    Plan {
        operations: vec![Operation::Rename { from, to, force }],
        changes: 1,
    }
}
