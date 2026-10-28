use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::Path;

use super::diff::Change;
use super::entry::EntryKind;
use super::session::DirectoryEdit;

type ChangeKey = (OsString, EntryKind);

#[derive(Clone, Copy)]
struct ChangeLocation {
    edit: usize,
    index: usize,
}

pub(super) fn merge_cross_directory_moves(edits: &mut [DirectoryEdit]) {
    let (deletes, creates) = collect_locations(edits);
    let pairs = matching_pairs(&deletes, &creates);
    let mut removals: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut moves = Vec::new();
    for (source, target) in pairs {
        let Some((edit, change)) = cross_directory_move(edits, source, target) else {
            continue;
        };
        removals
            .entry(source.edit)
            .or_default()
            .insert(source.index);
        removals
            .entry(target.edit)
            .or_default()
            .insert(target.index);
        moves.push((edit, change));
    }
    if moves.is_empty() {
        return;
    }
    remove_locations(edits, &removals);
    for (edit, change) in moves {
        edits[edit].changes.push(change);
    }
}

fn collect_locations(
    edits: &[DirectoryEdit],
) -> (
    BTreeMap<ChangeKey, Vec<ChangeLocation>>,
    BTreeMap<ChangeKey, Vec<ChangeLocation>>,
) {
    let mut deletes = BTreeMap::new();
    let mut creates = BTreeMap::new();
    for (edit, directory_edit) in edits.iter().enumerate() {
        for (index, change) in directory_edit.changes.iter().enumerate() {
            let location = ChangeLocation { edit, index };
            match change {
                Change::Delete { path, kind, .. } => {
                    insert_location(&mut deletes, path, *kind, location);
                }
                Change::Create { path, kind } => {
                    insert_location(&mut creates, path, *kind, location);
                }
                _ => {}
            }
        }
    }
    (deletes, creates)
}

fn insert_location(
    locations: &mut BTreeMap<ChangeKey, Vec<ChangeLocation>>,
    path: &Path,
    kind: EntryKind,
    location: ChangeLocation,
) {
    let Some(name) = path.file_name() else {
        return;
    };
    locations
        .entry((name.to_owned(), kind))
        .or_default()
        .push(location);
}

fn matching_pairs(
    deletes: &BTreeMap<ChangeKey, Vec<ChangeLocation>>,
    creates: &BTreeMap<ChangeKey, Vec<ChangeLocation>>,
) -> Vec<(ChangeLocation, ChangeLocation)> {
    let mut pairs = Vec::new();
    for (key, sources) in deletes {
        let [source] = sources.as_slice() else {
            continue;
        };
        let Some(targets) = creates.get(key) else {
            continue;
        };
        let [target] = targets.as_slice() else {
            continue;
        };
        if source.edit != target.edit {
            pairs.push((*source, *target));
        }
    }
    pairs
}

fn cross_directory_move(
    edits: &[DirectoryEdit],
    source: ChangeLocation,
    target: ChangeLocation,
) -> Option<(usize, Change)> {
    let Change::Delete {
        id,
        path: from,
        kind,
    } = &edits[source.edit].changes[source.index]
    else {
        return None;
    };
    let Change::Create { path: to, .. } = &edits[target.edit].changes[target.index] else {
        return None;
    };
    if from.parent() == to.parent() {
        return None;
    }
    let change = Change::Rename {
        id: *id,
        from: from.clone(),
        to: to.clone(),
        kind: *kind,
    };
    Some((source.edit, change))
}

fn remove_locations(edits: &mut [DirectoryEdit], removals: &BTreeMap<usize, BTreeSet<usize>>) {
    for (edit_index, edit) in edits.iter_mut().enumerate() {
        let removed = removals.get(&edit_index);
        let changes = std::mem::take(&mut edit.changes);
        edit.changes = changes
            .into_iter()
            .enumerate()
            .filter(|(index, _)| removed.is_none_or(|removed| !removed.contains(index)))
            .map(|(_, change)| change)
            .collect();
    }
}
