use std::collections::HashSet;

use ropey::Rope;

use super::*;
use crate::history_nav::UndoOffset;

fn rope(text: &str) -> Rope {
    Rope::from_str(text)
}

fn checkpoint(history: &mut History, text: &mut Rope, next: &str) {
    history.checkpoint(text, 0, 0, 0);
    *text = rope(next);
}

fn branched_history() -> (History, Rope) {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    checkpoint(&mut history, &mut text, "b");
    history.mark_clean();
    history.undo(&mut text, 0, 0, 0).unwrap();
    checkpoint(&mut history, &mut text, "c");
    (history, text)
}

fn assert_links(persisted: &PersistedHistory) {
    let ids: HashSet<u64> = persisted.entries.iter().map(|entry| entry.id).collect();
    let mut roots = 0;
    for entry in &persisted.entries {
        match entry.parent {
            Some(parent) => {
                assert!(ids.contains(&parent), "orphan {}", entry.id);
                assert!(parent < entry.id);
            }
            None => roots += 1,
        }
        if let Some(child) = entry.last_visited_child {
            assert_eq!(
                persisted
                    .entries
                    .iter()
                    .find(|candidate| candidate.id == child)
                    .unwrap()
                    .parent,
                Some(entry.id)
            );
        }
    }
    assert_eq!(roots, 1);
    assert!(ids.contains(&persisted.current_id));
    if let Some(clean) = persisted.clean_id {
        assert!(ids.contains(&clean));
    }
}

#[test]
fn round_trips_a_tree_and_current_pointer() {
    let (history, current) = branched_history();
    let persisted = history.export(&current, 0, 0, 0);
    assert_links(&persisted);
    let restored = History::from_persisted(&current, &persisted).expect("restore");
    assert_eq!(restored.current_id(), history.current_id());
    assert_eq!(restored.is_dirty(), history.is_dirty());
    assert_eq!(restored.nodes.len(), history.nodes.len());
}

#[test]
fn round_trips_last_visited_child_after_restart() {
    let (mut history, mut current) = branched_history();
    let steps = history.path_to(2).unwrap();
    for step in steps {
        match step {
            crate::history_nav::HistoryStep::Undo(id) => {
                history.undo_to(&mut current, 0, 0, 0, id).unwrap();
            }
            crate::history_nav::HistoryStep::Redo(id) => {
                history.redo_to(&mut current, 0, 0, 0, id).unwrap();
            }
        }
    }
    history.undo(&mut current, 0, 0, 0).unwrap();
    let persisted = history.export(&current, 0, 0, 0);
    let mut restored = History::from_persisted(&current, &persisted).unwrap();
    assert!(restored.redo(&mut current, 0, 0, 0).is_some());
    assert_eq!(current.to_string(), "b");
}

#[test]
fn round_trips_save_markers_and_write_counts() {
    let (history, current) = branched_history();
    let persisted = history.export(&current, 0, 0, 0);
    let restored = History::from_persisted(&current, &persisted).unwrap();
    assert_eq!(
        restored.offset_target(UndoOffset::FileWrites(1), false),
        Some(2)
    );
    assert_eq!(restored.next_write_marker, history.next_write_marker);
    assert_eq!(restored.next_id, history.next_id);
}

#[test]
fn import_enforces_the_in_memory_cap() {
    let mut history = History::new();
    let mut text = rope("0");
    for index in 1..=crate::history::MAX_HISTORY + 20 {
        checkpoint(&mut history, &mut text, &index.to_string());
    }
    let persisted = history.export(&text, 0, 0, 0);
    let restored = History::from_persisted(&text, &persisted).expect("restore");
    assert!(restored.nodes.len() <= crate::history::MAX_HISTORY);
    assert!(restored.nodes.contains_key(&restored.current_id));
}

#[test]
fn rejects_a_missing_parent() {
    let (history, current) = branched_history();
    let mut persisted = history.export(&current, 0, 0, 0);
    persisted.entries[1].parent = Some(999);
    assert!(History::from_persisted(&current, &persisted).is_none());
}

#[test]
fn rejects_a_parent_cycle() {
    let (history, current) = branched_history();
    let mut persisted = history.export(&current, 0, 0, 0);
    persisted.entries[0].parent = Some(3);
    assert!(History::from_persisted(&current, &persisted).is_none());
}

#[test]
fn rejects_a_missing_current_node() {
    let (history, current) = branched_history();
    let mut persisted = history.export(&current, 0, 0, 0);
    persisted.current_id = 999;
    assert!(History::from_persisted(&current, &persisted).is_none());
}

#[test]
fn rejects_tampered_deltas() {
    let (history, current) = branched_history();
    let mut persisted = history.export(&current, 0, 0, 0);
    persisted.entries[1].delta.inserted = "wrong".to_string();
    assert!(History::from_persisted(&current, &persisted).is_none());
}

#[test]
fn trimming_never_orphans_a_node() {
    let (history, current) = branched_history();
    let mut persisted = history.export(&current, 0, 0, 0);
    while persisted.trim_oldest() {
        assert_links(&persisted);
        let restored = History::from_persisted(&current, &persisted);
        assert!(restored.is_some());
        assert!(restored.unwrap().nodes.contains_key(&persisted.current_id));
    }
    assert_links(&persisted);
}

#[test]
fn handles_unicode_deltas() {
    let mut history = History::new();
    let mut text = rope("a𝄞b\n");
    history.checkpoint(&text, 0, 0, 0);
    text = rope("a𝄢b\n");
    let persisted = history.export(&text, 0, 0, 0);
    let mut restored = History::from_persisted(&text, &persisted).expect("restore");
    assert!(restored.undo(&mut text, 0, 0, 0).is_some());
    assert_eq!(text.to_string(), "a𝄞b\n");
}
