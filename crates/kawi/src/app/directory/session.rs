use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use super::super::save::SaveAfter;
use super::diff::{Change, DirectoryLine};
use super::entry::Entry;
use super::listing::Listing;
use super::metadata::{MetadataColumns, MetadataWidths};
use super::name::escape_name;
use super::planner::Plan;
use super::sort::SortOptions;
use super::watch::DirectoryWatch;
use matching::{entry_positions, match_existing, original_kinds, pair_renames, remember_removed};

mod matching;

#[derive(Default)]
pub(in crate::app) struct DirectoryBuffers {
    pub(super) states: BTreeMap<usize, DirectoryBuffer>,
    pending: Option<PendingApply>,
    pub(super) wake: Option<Sender<()>>,
    pub(super) watch: Option<DirectoryWatch>,
    pub(super) watch_failed: bool,
}

pub(in crate::app) struct DirectoryEdit {
    pub(in crate::app) number: usize,
    pub(in crate::app) changes: Vec<Change>,
}

pub(in crate::app) struct PendingApply {
    pub(in crate::app) edits: Vec<DirectoryEdit>,
    pub(in crate::app) plan: Plan,
    pub(in crate::app) after: SaveAfter,
    pub(in crate::app) visible: bool,
    pub(in crate::app) save_files: bool,
}

pub(in crate::app) struct DirectoryBuffer {
    pub(super) root: PathBuf,
    pub(super) show_hidden: bool,
    pub(super) sort: SortOptions,
    pub(super) original: Vec<Entry>,
    pub(super) lines: Vec<DirectoryLine>,
    pub(super) next_id: u64,
    pub(super) revision: u64,
    pub(super) removed: BTreeMap<String, Vec<u64>>,
    pub(super) positions: HashMap<u64, usize>,
    pub(super) metadata_widths: MetadataWidths,
    pub(super) external_changed: bool,
    pub(super) view_options_pending: bool,
}

impl DirectoryBuffers {
    pub(super) fn insert(&mut self, number: usize, state: DirectoryBuffer) {
        self.states.insert(number, state);
    }

    pub(in crate::app) fn get(&self, number: usize) -> Option<&DirectoryBuffer> {
        self.states.get(&number)
    }

    pub(super) fn get_mut(&mut self, number: usize) -> Option<&mut DirectoryBuffer> {
        self.states.get_mut(&number)
    }

    pub(in crate::app) fn contains(&self, number: usize) -> bool {
        self.states.contains_key(&number)
    }

    pub(super) fn remove(&mut self, number: usize) {
        self.states.remove(&number);
    }

    pub(in crate::app) fn retain_listed(&mut self, listed: &[usize]) {
        self.states.retain(|number, _| listed.contains(number));
    }

    pub(in crate::app) fn mark_external_change(&mut self, number: usize) -> bool {
        let Some(state) = self.states.get_mut(&number) else {
            return false;
        };
        if state.external_changed {
            return false;
        }
        state.external_changed = true;
        true
    }

    pub(in crate::app) fn clear_external_change(&mut self, number: usize) {
        if let Some(state) = self.states.get_mut(&number) {
            state.external_changed = false;
        }
    }

    pub(in crate::app) fn external_changed(&self, number: usize) -> bool {
        self.states
            .get(&number)
            .is_some_and(|state| state.external_changed)
    }

    pub(super) fn numbers(&self) -> impl Iterator<Item = usize> + '_ {
        self.states.keys().copied()
    }

    pub(super) fn find_root(&self, root: &Path) -> Option<usize> {
        self.states
            .iter()
            .find(|(_, state)| state.root == root)
            .map(|(number, _)| *number)
    }

    pub(in crate::app) fn remove_under(&mut self, target: &Path) -> Vec<usize> {
        let numbers: Vec<usize> = self
            .states
            .iter()
            .filter(|(_, state)| state.root.starts_with(target))
            .map(|(number, _)| *number)
            .collect();
        for number in &numbers {
            self.states.remove(number);
        }
        numbers
    }

    pub(super) fn set_pending(&mut self, pending: PendingApply) {
        self.pending = Some(pending);
    }

    pub(in crate::app) fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(super) fn take_pending(&mut self) -> Option<PendingApply> {
        self.pending.take()
    }

    pub(in crate::app) fn rewrite_path(&mut self, old: &Path, new: &Path) {
        for state in self.states.values_mut() {
            if let Ok(rest) = state.root.strip_prefix(old) {
                state.root = new.join(rest);
            }
        }
    }
}

impl DirectoryBuffer {
    pub(super) fn new(
        root: PathBuf,
        listing: Listing,
        show_hidden: bool,
        sort: SortOptions,
        revision: u64,
    ) -> Self {
        let positions = entry_positions(&listing.entries);
        let mut state = Self {
            root,
            show_hidden,
            sort,
            original: listing.entries,
            lines: Vec::new(),
            next_id: listing.next_id,
            revision,
            removed: BTreeMap::new(),
            positions,
            metadata_widths: listing.metadata_widths,
            external_changed: false,
            view_options_pending: false,
        };
        state.reset_lines();
        state
    }

    pub(super) fn sync(&mut self, texts: Vec<String>, revision: u64) {
        self.revision = revision;
        let old_lines = std::mem::take(&mut self.lines);
        let (mut ids, mut used) = match_existing(&old_lines, &texts, &mut self.removed);
        let kinds = original_kinds(&self.original);
        pair_renames(&old_lines, &texts, &mut ids, &mut used, &kinds);
        remember_removed(&old_lines, &used, &mut self.removed);
        self.lines = texts
            .into_iter()
            .zip(ids)
            .map(|(text, id)| DirectoryLine { id, text })
            .collect();
    }

    pub(super) fn reset_listing(&mut self, listing: Listing, revision: u64) {
        self.original = listing.entries;
        self.positions = entry_positions(&self.original);
        self.next_id = listing.next_id;
        self.metadata_widths = listing.metadata_widths;
        self.revision = revision;
        self.external_changed = false;
        self.removed.clear();
        self.reset_lines();
    }

    pub(super) fn root_display(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }

    pub(in crate::app) fn metadata_width(&self, columns: MetadataColumns) -> usize {
        columns.width(self.metadata_widths)
    }

    pub(in crate::app) fn metadata_text(
        &self,
        row: usize,
        columns: MetadataColumns,
    ) -> Option<String> {
        self.entry_at_row(row)?
            .metadata
            .as_ref()?
            .text(columns, self.metadata_widths)
    }

    pub(in crate::app) fn symlink_target(&self, row: usize) -> Option<String> {
        let target = self.entry_at_row(row)?.symlink_target.as_ref()?;
        Some(escape_name(target.as_os_str()))
    }

    fn entry_at_row(&self, row: usize) -> Option<&Entry> {
        let line = self.lines.get(row)?;
        if line.text.is_empty() {
            return None;
        }
        let position = *self.positions.get(&line.id?)?;
        self.original.get(position)
    }

    fn reset_lines(&mut self) {
        self.lines = self
            .original
            .iter()
            .map(|entry| DirectoryLine {
                id: Some(entry.id),
                text: entry.display_name(),
            })
            .collect();
    }
}
