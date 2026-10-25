use std::collections::{BTreeMap, HashMap, HashSet};

use ropey::Rope;

use crate::diff::diff_ropes;
use crate::history_persist::{HistoryDelta, HistoryEntry, PersistedHistory};

pub(super) fn delta_between(previous: &Rope, current: &Rope) -> HistoryDelta {
    let Some(change) = diff_ropes(previous, current) else {
        return HistoryDelta::default();
    };
    HistoryDelta {
        start: change.start,
        removed: previous.slice(change.start..change.old_end).to_string(),
        inserted: current.slice(change.start..change.new_end).to_string(),
    }
}

pub(super) fn validate_entries(
    persisted: &PersistedHistory,
) -> Option<HashMap<u64, &HistoryEntry>> {
    if persisted.entries.is_empty() {
        return None;
    }
    let entries: HashMap<u64, &HistoryEntry> = persisted
        .entries
        .iter()
        .map(|entry| (entry.id, entry))
        .collect();
    if entries.len() != persisted.entries.len() {
        return None;
    }
    if !entries.contains_key(&persisted.current_id) {
        return None;
    }
    if persisted
        .clean_id
        .is_some_and(|id| !entries.contains_key(&id))
    {
        return None;
    }
    let mut roots = 0;
    for entry in &persisted.entries {
        if let Some(parent) = entry.parent {
            if parent >= entry.id || !entries.contains_key(&parent) {
                return None;
            }
        } else {
            roots += 1;
        }
        if let Some(child) = entry.last_visited_child {
            if entries.get(&child)?.parent != Some(entry.id) {
                return None;
            }
        }
    }
    if roots != 1 {
        return None;
    }
    let mut visited = HashSet::new();
    let root = persisted
        .entries
        .iter()
        .find(|entry| entry.parent.is_none())?
        .id;
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !visited.insert(id) {
            return None;
        }
        stack.extend(
            persisted
                .entries
                .iter()
                .filter(|entry| entry.parent == Some(id))
                .map(|entry| entry.id),
        );
    }
    (visited.len() == entries.len()).then_some(entries)
}

pub(super) fn reconstruct_texts(
    current: &Rope,
    persisted: &PersistedHistory,
    entries: &HashMap<u64, &HistoryEntry>,
) -> Option<BTreeMap<u64, Rope>> {
    let mut texts = BTreeMap::new();
    texts.insert(persisted.current_id, current.clone());
    let mut id = persisted.current_id;
    loop {
        let entry = entries.get(&id)?;
        let Some(parent) = entry.parent else {
            break;
        };
        let child_text = texts.get(&id)?;
        let parent_text = reverse_delta(child_text, &entry.delta)?;
        texts.insert(parent, parent_text);
        id = parent;
    }
    let mut ordered: Vec<&HistoryEntry> = persisted.entries.iter().collect();
    ordered.sort_by_key(|entry| entry.id);
    for entry in ordered {
        if texts.contains_key(&entry.id) {
            continue;
        }
        let parent = entry.parent?;
        let parent_text = texts.get(&parent)?;
        texts.insert(entry.id, forward_delta(parent_text, &entry.delta)?);
    }
    (texts.len() == entries.len()).then_some(texts)
}

fn forward_delta(text: &Rope, delta: &HistoryDelta) -> Option<Rope> {
    replace_delta(text, delta.start, &delta.removed, &delta.inserted)
}

fn reverse_delta(text: &Rope, delta: &HistoryDelta) -> Option<Rope> {
    replace_delta(text, delta.start, &delta.inserted, &delta.removed)
}

fn replace_delta(text: &Rope, start: usize, removed: &str, inserted: &str) -> Option<Rope> {
    let end = start.saturating_add(removed.chars().count());
    if start > text.len_chars() || end > text.len_chars() {
        return None;
    }
    if !text.slice(start..end).chars().eq(removed.chars()) {
        return None;
    }
    let mut result = text.clone();
    result.remove(start..end);
    result.insert(start, inserted);
    Some(result)
}
