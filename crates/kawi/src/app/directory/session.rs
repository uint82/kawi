use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::{Path, PathBuf};

use super::super::save::SaveAfter;
use super::diff::{Change, DirectoryLine};
use super::entry::Entry;
use super::entry::EntryKind;
use super::listing::{Listing, SortMode};
use super::metadata::{MetadataColumns, MetadataWidths};
use super::planner::Plan;

#[derive(Default)]
pub(in crate::app) struct DirectoryBuffers {
    states: BTreeMap<usize, DirectoryBuffer>,
    pending: Option<PendingApply>,
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
    pub(super) sort: SortMode,
    pub(super) original: Vec<Entry>,
    pub(super) lines: Vec<DirectoryLine>,
    pub(super) next_id: u64,
    pub(super) revision: u64,
    pub(super) removed: BTreeMap<String, Vec<u64>>,
    pub(super) positions: HashMap<u64, usize>,
    pub(super) metadata_widths: MetadataWidths,
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
        sort: SortMode,
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
        let line = self.lines.get(row)?;
        if line.text.is_empty() {
            return None;
        }
        let position = *self.positions.get(&line.id?)?;
        self.original
            .get(position)?
            .metadata
            .as_ref()?
            .text(columns, self.metadata_widths)
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

fn match_existing(
    old_lines: &[DirectoryLine],
    texts: &[String],
    removed: &mut BTreeMap<String, Vec<u64>>,
) -> (Vec<Option<u64>>, BTreeSet<u64>) {
    let mut available: HashMap<String, VecDeque<u64>> = HashMap::new();
    for line in old_lines {
        if let Some(id) = line.id {
            available
                .entry(line.text.clone())
                .or_default()
                .push_back(id);
        }
    }
    let mut first_by_text: HashMap<String, u64> = HashMap::new();
    let mut used = BTreeSet::new();
    let mut ids = Vec::with_capacity(texts.len());
    for text in texts {
        let id = available
            .get_mut(text)
            .and_then(VecDeque::pop_front)
            .or_else(|| take_removed(removed, text))
            .or_else(|| first_by_text.get(text).copied());
        if let Some(id) = id {
            used.insert(id);
            first_by_text.entry(text.clone()).or_insert(id);
        }
        ids.push(id);
    }
    (ids, used)
}

fn pair_renames(
    old_lines: &[DirectoryLine],
    texts: &[String],
    ids: &mut [Option<u64>],
    used: &mut BTreeSet<u64>,
    kinds: &HashMap<u64, EntryKind>,
) {
    for (index, id) in ids.iter_mut().enumerate() {
        if id.is_some() || texts[index].is_empty() {
            continue;
        }
        let Some(old) = old_lines.get(index) else {
            continue;
        };
        let Some(old_id) = old.id else {
            continue;
        };
        let Some(kind) = kinds.get(&old_id) else {
            continue;
        };
        if kind_matches_text(*kind, &texts[index]) {
            *id = Some(old_id);
            used.insert(old_id);
        }
    }
}

fn entry_positions(entries: &[Entry]) -> HashMap<u64, usize> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.id, index))
        .collect()
}

fn original_kinds(entries: &[Entry]) -> HashMap<u64, EntryKind> {
    entries.iter().map(|entry| (entry.id, entry.kind)).collect()
}

fn kind_matches_text(kind: EntryKind, text: &str) -> bool {
    (kind == EntryKind::Directory) == text.ends_with('/')
}

fn remember_removed(
    old_lines: &[DirectoryLine],
    used: &BTreeSet<u64>,
    removed: &mut BTreeMap<String, Vec<u64>>,
) {
    for line in old_lines {
        if let Some(id) = line.id {
            if !used.contains(&id) {
                let ids = removed.entry(line.text.clone()).or_default();
                if ids.len() < 32 && !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
    }
}

fn take_removed(removed: &mut BTreeMap<String, Vec<u64>>, text: &str) -> Option<u64> {
    let ids = removed.get_mut(text)?;
    let id = ids.pop();
    if ids.is_empty() {
        removed.remove(text);
    }
    id
}
