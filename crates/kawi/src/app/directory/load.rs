use ropey::Rope;

use super::diff::Change;
use super::listing::Listing;
use super::loader::LoadedListing;
use super::loading::{LoadOutcome, LoadPurpose};
use crate::app::save::SaveAfter;
use crate::app::App;

impl App {
    pub(in crate::app) fn poll_directory_loads(&mut self) -> bool {
        let outcomes = self.directory_loading.drain();
        let mut changed = false;
        for outcome in outcomes {
            changed |= self.apply_directory_load(outcome);
        }
        changed
    }

    pub(super) fn start_directory_initial_load(&mut self, number: usize) -> Result<(), String> {
        self.start_directory_display_load(number, true, false)
    }

    pub(in crate::app) fn start_directory_reload(&mut self, number: usize) -> Result<(), String> {
        self.start_directory_display_load(number, false, false)
    }

    pub(super) fn start_directory_reload_after_save(
        &mut self,
        number: usize,
    ) -> Result<(), String> {
        self.start_directory_display_load(number, false, true)
    }

    fn start_directory_display_load(
        &mut self,
        number: usize,
        initial: bool,
        allow_dirty: bool,
    ) -> Result<(), String> {
        if self.directory_loading.is_loading(number) {
            return Err("Directory is still loading".to_string());
        }
        if !allow_dirty && self.editor.buffer_is_dirty(number).unwrap_or(false) {
            return Err("Save or undo changes before reloading the directory".to_string());
        }
        self.start_directory_load(number, LoadPurpose::Display { initial })
    }

    pub(super) fn start_directory_save_check(
        &mut self,
        number: usize,
        changes: Vec<Change>,
        force: bool,
        after: SaveAfter,
        visible: bool,
    ) -> Result<(), String> {
        if self.directory_loading.is_loading(number) {
            return Err("Directory is still loading".to_string());
        }
        self.start_directory_load(
            number,
            LoadPurpose::Save {
                changes,
                force,
                after,
                visible,
            },
        )
    }

    fn start_directory_load(&mut self, number: usize, purpose: LoadPurpose) -> Result<(), String> {
        let (root, show_hidden, sort) = self
            .directory
            .get(number)
            .map(|state| (state.root.clone(), state.show_hidden, state.sort))
            .ok_or_else(|| "directory buffer is missing".to_string())?;
        self.directory_loading
            .submit(number, root, show_hidden, sort, purpose)?;
        self.editor.set_buffer_readonly(number, true);
        Ok(())
    }

    pub(in crate::app) fn cancel_directory_apply(&mut self, number: usize) {
        self.editor.set_buffer_readonly(number, false);
    }

    fn apply_directory_load(&mut self, outcome: LoadOutcome) -> bool {
        let LoadOutcome { pending, listing } = outcome;
        let number = pending.number;
        let same_root = self
            .directory
            .get(number)
            .is_some_and(|state| state.root == pending.root);
        if !same_root {
            self.editor.set_buffer_readonly(number, false);
            if self.directory.contains(number) {
                let _ = self.start_directory_reload(number);
            }
            return false;
        }
        if self.editor.buffer_is_directory(number).is_none() {
            self.editor.set_buffer_readonly(number, false);
            return false;
        }
        match pending.purpose {
            LoadPurpose::Display { initial } => {
                self.finish_directory_display_load(number, listing, initial)
            }
            LoadPurpose::Save {
                changes,
                force,
                after,
                visible,
            } => self.finish_directory_save_check(number, changes, force, after, visible, listing),
        }
    }

    fn finish_directory_display_load(
        &mut self,
        number: usize,
        listing: Result<LoadedListing, String>,
        initial: bool,
    ) -> bool {
        let Some(loaded) = self.accept_directory_listing(number, listing, initial) else {
            return true;
        };
        let LoadedListing { listing, text } = loaded;
        let Some(text) = text else {
            return false;
        };
        self.store_directory_listing(number, listing, text)
    }

    fn accept_directory_listing(
        &mut self,
        number: usize,
        listing: Result<LoadedListing, String>,
        initial: bool,
    ) -> Option<LoadedListing> {
        match listing {
            Ok(listing) => {
                self.editor.set_buffer_readonly(number, false);
                Some(listing)
            }
            Err(message) => {
                if !initial {
                    self.editor.set_buffer_readonly(number, false);
                }
                self.directory_loading.clear_focus(number);
                self.show_message(message);
                None
            }
        }
    }

    fn store_directory_listing(&mut self, number: usize, listing: Listing, text: Rope) -> bool {
        let focus = self.directory_loading.take_focus(number);
        let view = self.windows.view_for_buffer(number);
        let Some(root) = self.directory.get(number).map(|state| state.root_display()) else {
            return false;
        };
        if !self.editor.replace_directory_listing(number, &root, text) {
            return false;
        }
        let revision = self.editor.buffer_revision_at(number).unwrap_or_default();
        if let Some(state) = self.directory.get_mut(number) {
            state.reset_listing(listing, revision);
        }
        let focused = focus.is_some_and(|focus| self.focus_directory_line(number, &focus));
        if !focused && number == self.editor.buffer_number() {
            if let Some(view) = view {
                self.editor.restore_view_state(view);
            }
            self.reveal_cursor();
        }
        true
    }
}
