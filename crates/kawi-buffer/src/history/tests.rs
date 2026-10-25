use std::collections::HashMap;

use super::test_support::{apply_path, assert_integrity, checkpoint, rope};
use super::*;
use crate::history_nav::UndoOffset;

#[test]
fn fresh_is_clean() {
    let history = History::new();
    assert!(!history.is_dirty());
}

#[test]
fn checkpoint_marks_dirty() {
    let mut history = History::new();
    history.checkpoint(&rope("ab"), 0, 0, 0);
    assert!(history.is_dirty());
}

#[test]
fn undo_restores_snapshot_and_cursor() {
    let mut history = History::new();
    let before = rope("ab");
    history.checkpoint(&before, 0, 1, 0);
    let mut now = rope("abcd");
    assert_eq!(history.undo(&mut now, 0, 3, 0), Some(((0, 1), None)));
    assert_eq!(now.to_string(), before.to_string());
}

#[test]
fn redo_restores_forward_and_cursor() {
    let mut history = History::new();
    history.checkpoint(&rope("ab"), 0, 0, 0);
    let mut now = rope("abcd");
    history.undo(&mut now, 0, 3, 0);
    assert_eq!(history.redo(&mut now, 0, 0, 0), Some(((0, 3), None)));
    assert_eq!(now.to_string(), "abcd");
}

#[test]
fn undo_after_branching_keeps_the_old_branch() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    checkpoint(&mut history, &mut text, "b");
    history.undo(&mut text, 0, 0, 0).unwrap();
    assert_eq!(text.to_string(), "a");
    checkpoint(&mut history, &mut text, "c");
    assert_eq!(history.current_id(), 3);
    assert_eq!(apply_path(&mut history, &mut text, 2), Some(()));
    assert_eq!(text.to_string(), "b");
    assert_eq!(apply_path(&mut history, &mut text, 3), Some(()));
    assert_eq!(text.to_string(), "c");
}

#[test]
fn chronological_paths_round_trip_branches() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    checkpoint(&mut history, &mut text, "b");
    history.undo(&mut text, 0, 0, 0).unwrap();
    checkpoint(&mut history, &mut text, "c");
    for target in [2, 1, 3, 2, 3] {
        assert_eq!(apply_path(&mut history, &mut text, target), Some(()));
    }
    assert_eq!(text.to_string(), "c");
}

#[test]
fn redo_follows_the_last_visited_child() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    checkpoint(&mut history, &mut text, "b");
    history.undo(&mut text, 0, 0, 0).unwrap();
    checkpoint(&mut history, &mut text, "c");
    apply_path(&mut history, &mut text, 2).unwrap();
    history.undo(&mut text, 0, 0, 0).unwrap();
    assert_eq!(text.to_string(), "a");
    history.redo(&mut text, 0, 0, 0).unwrap();
    assert_eq!(text.to_string(), "b");
}

#[test]
fn redo_follows_the_last_visited_child_after_restart() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    checkpoint(&mut history, &mut text, "b");
    history.undo(&mut text, 0, 0, 0).unwrap();
    checkpoint(&mut history, &mut text, "c");
    apply_path(&mut history, &mut text, 2).unwrap();
    history.undo(&mut text, 0, 0, 0).unwrap();
    let persisted = history.export(&text, 0, 0, 0);
    let mut restored = History::from_persisted(&text, &persisted).unwrap();
    assert_eq!(restored.current_id(), 1);
    assert!(restored.redo(&mut text, 0, 0, 0).is_some());
    assert_eq!(text.to_string(), "b");
}

#[test]
fn earlier_and_later_states_cross_branches() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    checkpoint(&mut history, &mut text, "b");
    history.undo(&mut text, 0, 0, 0).unwrap();
    checkpoint(&mut history, &mut text, "c");
    assert_eq!(history.chronological_target(false, 1), Some(2));
    assert_eq!(history.chronological_target(false, 2), Some(1));
    assert_eq!(history.chronological_target(true, 1), None);
    apply_path(&mut history, &mut text, 1).unwrap();
    assert_eq!(history.chronological_target(true, 1), Some(2));
}

#[test]
fn file_write_offsets_land_on_save_states() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    history.mark_clean();
    checkpoint(&mut history, &mut text, "b");
    history.mark_clean();
    checkpoint(&mut history, &mut text, "c");
    history.mark_clean();
    let target = history.offset_target(UndoOffset::FileWrites(1), false);
    assert_eq!(target, Some(2));
    apply_path(&mut history, &mut text, target.unwrap()).unwrap();
    assert_eq!(text.to_string(), "b");
    assert!(history.is_dirty());
    let target = history.offset_target(UndoOffset::FileWrites(1), true);
    assert_eq!(target, Some(3));
    apply_path(&mut history, &mut text, target.unwrap()).unwrap();
    assert_eq!(text.to_string(), "c");
    assert!(!history.is_dirty());
}

#[test]
fn file_write_offsets_clamp_to_the_edges() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    history.mark_clean();
    checkpoint(&mut history, &mut text, "b");
    history.mark_clean();
    assert_eq!(
        history.offset_target(UndoOffset::FileWrites(10), false),
        Some(0)
    );
    assert_eq!(
        history.offset_target(UndoOffset::FileWrites(10), true),
        Some(history.current_id())
    );
}

#[test]
fn unsaved_changes_go_back_to_the_last_write() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    history.mark_clean();
    checkpoint(&mut history, &mut text, "b");
    let target = history.offset_target(UndoOffset::FileWrites(1), false);
    assert_eq!(target, Some(1));
    apply_path(&mut history, &mut text, target.unwrap()).unwrap();
    assert!(!history.is_dirty());
}

#[test]
fn time_offsets_use_node_timestamps() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    checkpoint(&mut history, &mut text, "b");
    let current = history.nodes.get_mut(&history.current_id).unwrap();
    current.timestamp = 10_000;
    history.nodes.get_mut(&1).unwrap().timestamp = 5_000;
    history.nodes.get_mut(&0).unwrap().timestamp = 1_000;
    assert_eq!(
        history.offset_target(UndoOffset::Seconds(5), false),
        Some(1)
    );
    assert_eq!(history.offset_target(UndoOffset::Seconds(5), true), Some(2));
}

#[test]
fn saving_before_the_first_edit_records_the_root_marker() {
    let mut history = History::new();
    let base = rope("base");
    history.mark_clean_at(&base, 0);
    let mut text = base.clone();
    checkpoint(&mut history, &mut text, "a");
    assert!(history.is_dirty());
    let target = history.offset_target(UndoOffset::FileWrites(1), false);
    assert_eq!(target, Some(0));
    apply_path(&mut history, &mut text, target.unwrap()).unwrap();
    assert_eq!(text.to_string(), "base");
    assert!(!history.is_dirty());
}

#[test]
fn leaves_include_branches_and_save_markers() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    checkpoint(&mut history, &mut text, "b");
    history.mark_clean();
    history.undo(&mut text, 0, 0, 0).unwrap();
    checkpoint(&mut history, &mut text, "c");
    let leaves = history.leaves_at(now_millis());
    assert_eq!(leaves.len(), 2);
    assert!(leaves
        .iter()
        .any(|leaf| leaf.sequence == 2 && leaf.save_marker == Some(1)));
    assert!(leaves.iter().any(|leaf| leaf.sequence == 3 && leaf.current));
}

#[test]
fn group_records_once() {
    let mut history = History::new();
    let before = rope("ab");
    history.begin_change();
    history.maybe_checkpoint(&before, 0, 0, 0);
    history.maybe_checkpoint(&rope("abc"), 0, 1, 0);
    let mut now = rope("abcd");
    assert!(history.undo(&mut now, 0, 2, 0).is_some());
    assert_eq!(now.to_string(), before.to_string());
    assert_eq!(history.undo(&mut now, 0, 0, 0), None);
}

#[test]
fn empty_group_records_nothing() {
    let mut history = History::new();
    history.begin_change();
    history.end_change();
    let mut now = rope("ab");
    assert_eq!(history.undo(&mut now, 0, 0, 0), None);
}

#[test]
fn undo_to_saved_clears_dirty() {
    let mut history = History::new();
    history.checkpoint(&rope("ab"), 0, 0, 0);
    let mut now = rope("abc");
    history.mark_clean();
    history.checkpoint(&now, 0, 1, 0);
    now = rope("abcd");
    assert!(history.undo(&mut now, 0, 2, 0).is_some());
    assert_eq!(now.to_string(), "abc");
    assert!(!history.is_dirty());
}

#[test]
fn cursor_lands_in_the_changed_region_after_a_jump() {
    use crate::Buffer;

    let mut buffer = Buffer::from_string("base\n");
    buffer.checkpoint(0, 0);
    buffer.replace_all("one\ntwo\n");
    buffer.checkpoint(0, 0);
    buffer.replace_all("one\nchanged\n");
    buffer.checkpoint(0, 0);
    buffer.replace_all("tip\n");
    buffer.undo(0, 0).unwrap();
    let sequence = buffer.undo_leaves().first().map(|leaf| leaf.sequence);
    assert!(sequence.is_some());
    let old = buffer.undo_leaves().into_iter().find(|leaf| !leaf.current);
    if let Some(leaf) = old {
        let (row, col) = buffer.jump_undo_sequence(0, 0, leaf.sequence).unwrap();
        assert!(row < buffer.line_count());
        assert!(col <= buffer.line_len(row));
    }
}

#[test]
fn tree_parent_links_are_consistent() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    checkpoint(&mut history, &mut text, "b");
    history.undo(&mut text, 0, 0, 0).unwrap();
    checkpoint(&mut history, &mut text, "c");
    assert_integrity(&history);
    let parent_counts: HashMap<u64, usize> = history
        .nodes
        .values()
        .filter_map(|node| node.parent)
        .fold(HashMap::new(), |mut counts, parent| {
            *counts.entry(parent).or_default() += 1;
            counts
        });
    assert_eq!(parent_counts.get(&1), Some(&2));
}
