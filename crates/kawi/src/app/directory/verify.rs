use super::super::save::SaveAfter;
use super::diff::Change;
use super::entry::EntryKind;
use super::fs::RealFs;
use super::listing::listing_unchanged;
use super::loader::LoadedListing;
use super::planner::{plan, Plan};
use super::session::PendingApply;
use crate::app::App;

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
        if !force && self.directory_options.confirm && plan.needs_confirmation() {
            self.confirm_directory_apply(number, changes, plan, after, visible);
            return true;
        }
        self.begin_directory_apply(number, changes, plan, after, visible);
        true
    }

    fn verify_directory_listing(
        &mut self,
        number: usize,
        listing: Result<LoadedListing, String>,
    ) -> bool {
        let LoadedListing { listing, .. } = match listing {
            Ok(listing) => listing,
            Err(message) => {
                self.editor.set_buffer_readonly(number, false);
                self.show_message(message);
                return false;
            }
        };
        let unchanged = self
            .directory
            .get(number)
            .is_some_and(|state| listing_unchanged(&state.original, &listing));
        if unchanged {
            return true;
        }
        self.editor.set_buffer_readonly(number, false);
        self.show_message("Directory changed on disk; reload before saving".to_string());
        false
    }

    fn plan_directory_changes(&mut self, number: usize, changes: &[Change]) -> Option<Plan> {
        let fs = RealFs::new(!self.directory_options.trash);
        match plan(changes.to_vec(), &fs) {
            Ok(plan) => Some(plan),
            Err(message) => {
                self.editor.set_buffer_readonly(number, false);
                self.show_message(message);
                None
            }
        }
    }

    fn confirm_directory_apply(
        &mut self,
        number: usize,
        changes: Vec<Change>,
        plan: Plan,
        after: SaveAfter,
        visible: bool,
    ) {
        self.show_message(confirmation_message(&changes));
        self.directory.set_pending(PendingApply {
            number,
            changes,
            plan,
            after,
            visible,
        });
    }

    pub(super) fn apply_pending_directory(&mut self, accept: bool) {
        let Some(pending) = self.directory.take_pending() else {
            return;
        };
        if !accept {
            self.editor.set_buffer_readonly(pending.number, false);
            self.show_message("Directory changes cancelled".to_string());
            return;
        }
        self.begin_directory_apply(
            pending.number,
            pending.changes,
            pending.plan,
            pending.after,
            pending.visible,
        );
    }
}

fn confirmation_message(changes: &[Change]) -> String {
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
