use std::collections::BTreeSet;

use ropey::Rope;

use crate::diff::diff_ropes;
use crate::history::{History, Snapshot};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryDelta {
    pub start: usize,
    pub removed: String,
    pub inserted: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: u64,
    pub parent: Option<u64>,
    pub row: usize,
    pub col: usize,
    pub line_changes: u64,
    pub delta: HistoryDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedHistory {
    pub current_id: u64,
    pub entries: Vec<HistoryEntry>,
}

impl PersistedHistory {
    pub fn has_changes(&self) -> bool {
        self.entries.len() > 1
    }

    pub fn current_index(&self) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.id == self.current_id)
    }

    pub fn trim_oldest(&mut self) -> bool {
        let Some(current) = self.current_index() else {
            return false;
        };
        if current > 0 {
            self.entries.remove(0);
            self.entries[0].parent = None;
            self.entries[0].delta = HistoryDelta::default();
            return true;
        }
        if self.entries.len() > 1 {
            self.entries.pop();
            return true;
        }
        false
    }
}

impl History {
    pub fn export(
        &self,
        current: &Rope,
        row: usize,
        col: usize,
        line_changes: u64,
    ) -> PersistedHistory {
        let mut states: Vec<State<'_>> = self.undo.iter().map(State::from_snapshot).collect();
        states.push(State {
            id: self.current_id,
            text: current,
            row,
            col,
            line_changes,
        });
        states.extend(self.redo.iter().rev().map(State::from_snapshot));
        let mut entries = Vec::with_capacity(states.len());
        for (index, state) in states.iter().enumerate() {
            let (parent, delta) = match index.checked_sub(1).and_then(|i| states.get(i)) {
                Some(previous) => (Some(previous.id), delta_between(previous.text, state.text)),
                None => (None, HistoryDelta::default()),
            };
            entries.push(HistoryEntry {
                id: state.id,
                parent,
                row: state.row,
                col: state.col,
                line_changes: state.line_changes,
                delta,
            });
        }
        PersistedHistory {
            current_id: self.current_id,
            entries,
        }
    }

    pub fn from_persisted(current: &Rope, persisted: &PersistedHistory) -> Option<Self> {
        validate_entries(&persisted.entries)?;
        let current_index = persisted.current_index()?;
        let mut texts = reconstruct_texts(current, persisted, current_index)?;
        let next_id = persisted
            .entries
            .iter()
            .map(|entry| entry.id)
            .max()
            .unwrap_or_default()
            .saturating_add(1);
        let mut snapshots = persisted
            .entries
            .iter()
            .map(|entry| {
                Some(Snapshot {
                    id: entry.id,
                    text: texts.pop_front()?,
                    row: entry.row,
                    col: entry.col,
                    line_changes: entry.line_changes,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let mut redo = snapshots.split_off(current_index + 1);
        snapshots.pop()?;
        redo.reverse();
        Some(Self {
            undo: snapshots,
            redo,
            group_open: false,
            current_id: persisted.current_id,
            next_id,
            clean_id: persisted.current_id,
        })
    }
}

struct State<'a> {
    id: u64,
    text: &'a Rope,
    row: usize,
    col: usize,
    line_changes: u64,
}

impl<'a> State<'a> {
    fn from_snapshot(snapshot: &'a Snapshot) -> Self {
        Self {
            id: snapshot.id,
            text: &snapshot.text,
            row: snapshot.row,
            col: snapshot.col,
            line_changes: snapshot.line_changes,
        }
    }
}

fn delta_between(previous: &Rope, current: &Rope) -> HistoryDelta {
    let Some(change) = diff_ropes(previous, current) else {
        return HistoryDelta::default();
    };
    HistoryDelta {
        start: change.start,
        removed: previous.slice(change.start..change.old_end).to_string(),
        inserted: current.slice(change.start..change.new_end).to_string(),
    }
}

fn validate_entries(entries: &[HistoryEntry]) -> Option<()> {
    if entries.is_empty() {
        return None;
    }
    let mut ids = BTreeSet::new();
    for (index, entry) in entries.iter().enumerate() {
        if !ids.insert(entry.id) {
            return None;
        }
        let expected = index.checked_sub(1).map(|previous| entries[previous].id);
        if entry.parent != expected {
            return None;
        }
    }
    Some(())
}

fn reconstruct_texts(
    current: &Rope,
    persisted: &PersistedHistory,
    current_index: usize,
) -> Option<std::collections::VecDeque<Rope>> {
    let mut texts = vec![None; persisted.entries.len()];
    texts[current_index] = Some(current.clone());
    for index in (1..=current_index).rev() {
        let next = texts[index].as_ref()?;
        texts[index - 1] = Some(reverse_delta(next, &persisted.entries[index].delta)?);
    }
    for index in current_index + 1..persisted.entries.len() {
        let previous = texts[index - 1].as_ref()?;
        texts[index] = Some(forward_delta(previous, &persisted.entries[index].delta)?);
    }
    texts.into_iter().collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rope(text: &str) -> Rope {
        Rope::from_str(text)
    }

    fn history_with_changes() -> (History, Rope) {
        let mut history = History::new();
        history.checkpoint(&rope("one\n"), 0, 1, 0);
        history.checkpoint(&rope("two\n"), 0, 1, 1);
        (history, rope("three\n"))
    }

    #[test]
    fn round_trips_undo_and_redo_states() {
        let (mut history, current) = history_with_changes();
        let mut undone = current.clone();
        history.undo(&mut undone, 0, 0, 0);
        let persisted = history.export(&undone, 0, 0, 0);
        assert!(persisted.has_changes());
        let mut restored = History::from_persisted(&undone, &persisted).expect("restore");
        assert!(!restored.is_dirty());
        assert_eq!(restored.undo.len(), 1);
        assert_eq!(restored.redo.len(), 1);
        assert!(restored.redo(&mut undone, 0, 0, 0).is_some());
        assert_eq!(undone.to_string(), "three\n");
    }

    #[test]
    fn round_trips_a_linear_history() {
        let (history, current) = history_with_changes();
        let persisted = history.export(&current, 0, 0, 0);
        let mut restored = History::from_persisted(&current, &persisted).expect("restore");
        let mut text = current.clone();
        assert!(restored.undo(&mut text, 0, 0, 0).is_some());
        assert_eq!(text.to_string(), "two\n");
        assert!(restored.undo(&mut text, 0, 0, 0).is_some());
        assert_eq!(text.to_string(), "one\n");
        assert!(restored.undo(&mut text, 0, 0, 0).is_none());
    }

    #[test]
    fn rejects_tampered_deltas() {
        let (history, current) = history_with_changes();
        let mut persisted = history.export(&current, 0, 0, 0);
        persisted.entries[1].delta.inserted = "wrong".to_string();
        assert!(History::from_persisted(&current, &persisted).is_none());
    }

    #[test]
    fn trim_oldest_keeps_the_current_state() {
        let (history, current) = history_with_changes();
        let mut persisted = history.export(&current, 0, 0, 0);
        assert!(persisted.trim_oldest());
        assert_eq!(persisted.current_id, persisted.entries.last().unwrap().id);
        assert!(History::from_persisted(&current, &persisted).is_some());
    }

    #[test]
    fn handles_unicode_deltas() {
        let mut history = History::new();
        history.checkpoint(&rope("a𝄞b\n"), 0, 0, 0);
        let current = rope("a𝄢b\n");
        let persisted = history.export(&current, 0, 0, 0);
        let mut restored = History::from_persisted(&current, &persisted).expect("restore");
        let mut text = current.clone();
        assert!(restored.undo(&mut text, 0, 0, 0).is_some());
        assert_eq!(text.to_string(), "a𝄞b\n");
    }
}
