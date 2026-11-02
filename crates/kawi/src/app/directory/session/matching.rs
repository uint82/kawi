use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use super::super::diff::DirectoryLine;
use super::super::entry::{Entry, EntryKind};

pub(super) fn match_existing(
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

pub(super) fn pair_renames(
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

pub(super) fn entry_positions(entries: &[Entry]) -> HashMap<u64, usize> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.id, index))
        .collect()
}

pub(super) fn original_kinds(entries: &[Entry]) -> HashMap<u64, EntryKind> {
    entries.iter().map(|entry| (entry.id, entry.kind)).collect()
}

fn kind_matches_text(kind: EntryKind, text: &str) -> bool {
    (kind == EntryKind::Directory) == text.ends_with('/')
}

pub(super) fn remember_removed(
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
