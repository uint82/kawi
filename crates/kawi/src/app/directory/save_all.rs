use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use super::super::save::SaveAfter;
use super::diff::Change;
use super::permission_check::PermissionPhase;
use super::session::{DirectoryEdit, PendingApply};
use crate::app::App;

pub(in crate::app) struct PendingSaveAll {
    pub(super) batch: u64,
    pub(super) edits: BTreeMap<usize, Vec<Change>>,
    pub(super) awaiting: BTreeSet<usize>,
    pub(super) after: SaveAfter,
}

impl App {
    pub(in crate::app) fn start_directory_save_all(&mut self, after: SaveAfter) -> bool {
        if self.reject_running_permission_check(true) {
            return false;
        }
        let numbers = self.dirty_directory_numbers();
        if let Some(message) = self.loading_directory_message(&numbers) {
            self.show_message(message);
            return false;
        }
        let Some(edits) = self.save_all_edits(&numbers) else {
            return false;
        };
        if edits.is_empty() {
            return self.finish_save_all_files(after);
        }
        if let Err(message) = self.ensure_disjoint_roots(&edits) {
            self.show_message(message);
            return false;
        }
        self.submit_save_all(after, edits)
    }

    pub(super) fn check_save_all_loads(&mut self) {
        let Some(pending) = self.pending_save_all.as_ref() else {
            return;
        };
        if pending
            .awaiting
            .iter()
            .any(|number| !self.directory_loading.is_loading(*number))
        {
            self.abort_save_all("Directory save cancelled".to_string());
        }
    }

    pub(super) fn finish_directory_save_all_check(
        &mut self,
        number: usize,
        batch: u64,
        listing: Result<super::loader::LoadedListing, String>,
    ) -> bool {
        let Some(pending) = self.pending_save_all.as_ref() else {
            return false;
        };
        if pending.batch != batch || !pending.awaiting.contains(&number) {
            return false;
        }
        if let Err(message) = self.check_directory_listing(number, listing) {
            self.directory.mark_external_change(number);
            self.abort_save_all(message);
            return true;
        }
        let Some(pending) = self.pending_save_all.as_mut() else {
            return false;
        };
        pending.awaiting.remove(&number);
        if pending.awaiting.is_empty() {
            self.finish_save_all_checks();
        }
        true
    }

    fn loading_directory_message(&self, numbers: &[usize]) -> Option<String> {
        let number = numbers
            .iter()
            .find(|number| self.directory_loading.is_loading(**number))?;
        Some(format!(
            "Directory is still loading: {}",
            self.directory
                .get(*number)
                .map_or_else(String::new, |state| state.root_display())
        ))
    }

    fn submit_save_all(&mut self, after: SaveAfter, edits: BTreeMap<usize, Vec<Change>>) -> bool {
        let batch = self.next_save_batch_id();
        if let Err(message) = self.start_save_all_checks(batch, &edits) {
            self.show_message(message);
            return false;
        }
        let awaiting = edits.keys().copied().collect();
        self.pending_save_all = Some(PendingSaveAll {
            batch,
            edits,
            awaiting,
            after,
        });
        self.show_message("Checking directories...".to_string());
        false
    }

    fn save_all_edits(&mut self, numbers: &[usize]) -> Option<BTreeMap<usize, Vec<Change>>> {
        let mut edits = BTreeMap::new();
        for number in numbers {
            self.sync_directory_buffer(*number);
            let changes = match self.directory_changes(*number) {
                Ok(changes) => changes,
                Err(message) => {
                    self.show_message(message);
                    return None;
                }
            };
            if changes.is_empty() {
                self.editor.mark_buffer_clean(*number);
            } else {
                edits.insert(*number, changes);
            }
        }
        Some(edits)
    }

    fn ensure_disjoint_roots(&self, edits: &BTreeMap<usize, Vec<Change>>) -> Result<(), String> {
        let roots: Vec<PathBuf> = edits
            .keys()
            .filter_map(|number| self.directory.get(*number).map(|state| state.root.clone()))
            .collect();
        for (index, root) in roots.iter().enumerate() {
            if let Some(other) = roots
                .iter()
                .skip(index.saturating_add(1))
                .find(|other| root.starts_with(other) || other.starts_with(root))
            {
                return Err(format!(
                    "Nested directory buffers must be saved separately: {} and {}",
                    root.display(),
                    other.display()
                ));
            }
        }
        Ok(())
    }

    fn next_save_batch_id(&mut self) -> u64 {
        self.save_batch_id = self.save_batch_id.checked_add(1).unwrap_or(1);
        self.save_batch_id
    }

    fn start_save_all_checks(
        &mut self,
        batch: u64,
        edits: &BTreeMap<usize, Vec<Change>>,
    ) -> Result<(), String> {
        let mut started = Vec::new();
        for number in edits.keys() {
            if let Err(message) = self.start_directory_save_all_check(*number, batch) {
                for number in started {
                    self.editor.set_buffer_readonly(number, false);
                    self.directory_loading.forget(number);
                }
                return Err(message);
            }
            started.push(*number);
        }
        Ok(())
    }

    fn abort_save_all(&mut self, message: String) {
        let Some(pending) = self.pending_save_all.take() else {
            return;
        };
        for number in &pending.awaiting {
            self.directory_loading.forget(*number);
        }
        for number in pending.edits.keys() {
            self.editor.set_buffer_readonly(*number, false);
        }
        self.show_message(message);
    }

    fn finish_save_all_checks(&mut self) {
        let Some(pending) = self.pending_save_all.take() else {
            return;
        };
        let mut edits = pending_edits(pending.edits);
        super::moves::merge_cross_directory_moves(&mut edits);
        let changes = super::save::all_changes(&edits);
        let plan = match self.check_directory_plan(&changes) {
            Ok(plan) => plan,
            Err(message) => {
                self.cancel_directory_apply(&edits);
                self.show_message(message);
                return;
            }
        };
        self.apply_or_confirm_save_all(edits, plan, pending.after);
    }

    fn apply_or_confirm_save_all(
        &mut self,
        edits: Vec<DirectoryEdit>,
        plan: super::planner::Plan,
        after: SaveAfter,
    ) {
        let apply = PendingApply {
            edits,
            plan,
            after,
            visible: true,
            save_files: true,
        };
        let confirm = self.directory_options.confirm && apply.plan.needs_confirmation();
        self.start_permission_check(apply, PermissionPhase::Plan, confirm);
    }

    fn dirty_directory_numbers(&self) -> Vec<usize> {
        self.directory
            .numbers()
            .filter(|number| self.editor.buffer_is_dirty(*number).unwrap_or(false))
            .collect()
    }
}

fn pending_edits(edits: BTreeMap<usize, Vec<Change>>) -> Vec<DirectoryEdit> {
    edits
        .into_iter()
        .map(|(number, changes)| DirectoryEdit { number, changes })
        .collect()
}
