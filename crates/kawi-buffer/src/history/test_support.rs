use std::collections::HashSet;

use ropey::Rope;

use crate::history::History;
use crate::history_nav::HistoryStep;

pub(super) fn rope(text: &str) -> Rope {
    Rope::from_str(text)
}

pub(super) fn checkpoint(history: &mut History, text: &mut Rope, next: &str) {
    history.checkpoint(text, 0, 0, 0);
    *text = rope(next);
}

pub(super) fn apply_path(history: &mut History, text: &mut Rope, target: u64) -> Option<()> {
    let steps = history.path_to(target)?;
    for step in steps {
        let row = 0;
        let col = 0;
        match step {
            HistoryStep::Undo(id) => {
                history.undo_to(text, row, col, 0, id)?;
            }
            HistoryStep::Redo(id) => {
                history.redo_to(text, row, col, 0, id)?;
            }
        }
    }
    Some(())
}

pub(super) fn assert_integrity(history: &History) {
    let mut parents = HashSet::new();
    let mut roots = 0;
    for (id, node) in &history.nodes {
        match node.parent {
            Some(parent) => {
                assert!(history.nodes.contains_key(&parent), "orphan {id}");
                assert!(parent < *id, "non-monotonic parent for {id}");
                parents.insert(parent);
            }
            None => roots += 1,
        }
        if let Some(child) = node.last_visited_child {
            assert_eq!(history.nodes.get(&child).unwrap().parent, Some(*id));
        }
    }
    assert_eq!(roots, 1);
    assert!(history.nodes.contains_key(&history.current_id));
    let root = history
        .nodes
        .iter()
        .find(|(_, node)| node.parent.is_none())
        .map(|(id, _)| *id)
        .unwrap();
    let mut seen = HashSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        assert!(seen.insert(id), "cycle at {id}");
        stack.extend(
            history
                .nodes
                .iter()
                .filter(|(_, node)| node.parent == Some(id))
                .map(|(child, _)| *child),
        );
    }
    assert_eq!(seen.len(), history.nodes.len());
}
