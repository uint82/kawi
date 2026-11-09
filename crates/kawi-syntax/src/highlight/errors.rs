use std::collections::HashSet;
use std::ops::Range;

use tree_sitter::{Node, Tree};

use crate::diff::LineIndex;

pub(crate) fn error_rows(tree: &Tree, index: &LineIndex, window: &Range<usize>) -> HashSet<usize> {
    let mut rows = HashSet::new();
    if window.is_empty() {
        return rows;
    }
    let bounds = index.line_start(window.start)..index.line_start(window.end);
    let mut pending = vec![tree.root_node()];
    while let Some(node) = pending.pop() {
        if !node.has_error() {
            continue;
        }
        if node.is_error() || node.is_missing() {
            mark_rows(node, index, &bounds, &mut rows);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.has_error() {
                pending.push(child);
            }
        }
    }
    rows
}

fn mark_rows(node: Node<'_>, index: &LineIndex, bounds: &Range<usize>, rows: &mut HashSet<usize>) {
    let start = node.start_byte().max(bounds.start);
    let end = node.end_byte().min(bounds.end);
    if start > end {
        return;
    }
    if start == end {
        if bounds.contains(&start) {
            rows.insert(index.point(start).row);
        }
        return;
    }
    let first = index.point(start).row;
    let last = index.point(end.saturating_sub(1)).row;
    rows.extend(first..=last);
}
