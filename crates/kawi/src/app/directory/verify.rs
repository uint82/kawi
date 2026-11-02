use super::super::save::SaveAfter;
use super::diff::Change;
use super::entry::EntryKind;
use super::fs::RealFs;
use super::listing::listing_unchanged;
use super::loader::LoadedListing;
use super::permission_check::PermissionPhase;
use super::planner::{plan, Plan};
use super::session::{DirectoryEdit, PendingApply};
use crate::app::App;

pub(super) const EXTERNAL_CHANGE_MESSAGE: &str = "Directory changed on disk; reload before saving";

impl App {
    pub(super) fn finish_directory_save_check(
        &mut self,
        number: usize,
        changes: Vec<Change>,
        force: bool,
        after: SaveAfter,
        visible: bool,
        listing: Result<LoadedListing, String>,
    ) -> bool {
        if !self.verify_directory_listing(number, listing) {
            return true;
        }
        let Some(plan) = self.plan_directory_changes(number, &changes) else {
            return true;
        };
        let apply = PendingApply {
            edits: vec![DirectoryEdit { number, changes }],
            plan,
            after,
            visible,
            save_files: false,
        };
        let confirm = !force && self.directory_options.confirm && apply.plan.needs_confirmation();
        self.start_permission_check(apply, PermissionPhase::Plan, confirm);
        true
    }

    fn verify_directory_listing(
        &mut self,
        number: usize,
        listing: Result<LoadedListing, String>,
    ) -> bool {
        match self.check_directory_listing(number, listing) {
            Ok(()) => true,
            Err(message) => {
                self.directory.mark_external_change(number);
                self.editor.set_buffer_readonly(number, false);
                self.show_message(message);
                false
            }
        }
    }

    pub(super) fn check_directory_listing(
        &self,
        number: usize,
        listing: Result<LoadedListing, String>,
    ) -> Result<(), String> {
        let LoadedListing { listing, .. } = listing?;
        let unchanged = self
            .directory
            .get(number)
            .is_some_and(|state| listing_unchanged(&state.original, &listing));
        if unchanged {
            Ok(())
        } else {
            Err(EXTERNAL_CHANGE_MESSAGE.to_string())
        }
    }

    fn plan_directory_changes(&mut self, number: usize, changes: &[Change]) -> Option<Plan> {
        match self.check_directory_plan(changes) {
            Ok(plan) => Some(plan),
            Err(message) => {
                self.editor.set_buffer_readonly(number, false);
                self.show_message(message);
                None
            }
        }
    }

    pub(super) fn check_directory_plan(&self, changes: &[Change]) -> Result<Plan, String> {
        let fs = RealFs::new(!self.directory_options.trash);
        plan(changes.to_vec(), &fs)
    }

    pub(super) fn confirm_directory_apply(&mut self, apply: PendingApply) {
        let changes = apply
            .edits
            .iter()
            .flat_map(|edit| edit.changes.iter().cloned())
            .collect::<Vec<Change>>();
        self.show_message(confirmation_message(&changes));
        self.directory.set_pending(apply);
    }

    pub(super) fn apply_pending_directory(&mut self, accept: bool) {
        let Some(pending) = self.directory.take_pending() else {
            return;
        };
        if !accept {
            self.cancel_directory_apply(&pending.edits);
            self.show_message("Directory changes cancelled".to_string());
            return;
        }
        self.begin_directory_apply(pending);
    }
}

pub(super) fn confirmation_message(changes: &[Change]) -> String {
    let mut labels: Vec<String> = changes.iter().take(3).map(change_label).collect();
    if changes.len() > labels.len() {
        labels.push("...".to_string());
    }
    format!(
        "Apply {} change(s): {} [y/N]",
        changes.len(),
        labels.join(", ")
    )
}

fn change_label(change: &Change) -> String {
    match change {
        Change::Create { path, kind } => {
            let action = if *kind == EntryKind::Directory {
                "create directory"
            } else {
                "create"
            };
            format!("{action} {}", path.display())
        }
        Change::Rename { from, to, .. } => {
            format!("rename {} to {}", from.display(), to.display())
        }
        Change::Copy { from, to, .. } => {
            format!("copy {} to {}", from.display(), to.display())
        }
        Change::Delete { path, .. } => format!("delete {}", path.display()),
    }
}
