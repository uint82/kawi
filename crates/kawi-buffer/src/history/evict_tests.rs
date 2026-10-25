use crate::history::{History, MAX_HISTORY};

use super::test_support::{assert_integrity, checkpoint, rope};

#[test]
fn in_memory_cap_never_orphans_nodes() {
    let mut history = History::new();
    let mut text = rope("0");
    for index in 1..=MAX_HISTORY + 20 {
        checkpoint(&mut history, &mut text, &index.to_string());
        assert_integrity(&history);
    }
    assert_eq!(history.nodes.len(), MAX_HISTORY);
    assert!(history.current_id() > 0);
}

#[test]
fn in_memory_cap_evicts_old_branches_whole() {
    let mut history = History::new();
    let mut text = rope("base");
    checkpoint(&mut history, &mut text, "a");
    checkpoint(&mut history, &mut text, "b");
    history.undo(&mut text, 0, 0, 0).unwrap();
    checkpoint(&mut history, &mut text, "c");
    for index in 0..MAX_HISTORY {
        checkpoint(&mut history, &mut text, &format!("new{index}"));
    }
    assert_integrity(&history);
    assert!(history.nodes.len() <= MAX_HISTORY);
    assert!(!history.nodes.contains_key(&2));
}
