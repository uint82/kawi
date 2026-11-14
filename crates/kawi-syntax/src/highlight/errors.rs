use std::collections::HashSet;
use std::ops::Range;

use ropey::Rope;
use tree_sitter::{Node, Tree};

use crate::text;

pub(crate) fn error_rows(tree: &Tree, text: &Rope, window: &Range<usize>) -> HashSet<usize> {
    let mut rows = HashSet::new();
    if window.is_empty() {
        return rows;
    }
    let bounds = text::line_start(text, window.start)..text::line_start(text, window.end);
    let mut pending = vec![tree.root_node()];
    while let Some(node) = pending.pop() {
        if !node.has_error() {
            continue;
        }
        if node.is_error() || node.is_missing() {
            mark_rows(node, text, &bounds, &mut rows);
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

fn mark_rows(node: Node<'_>, text: &Rope, bounds: &Range<usize>, rows: &mut HashSet<usize>) {
    let start = node.start_byte().max(bounds.start);
    let end = node.end_byte().min(bounds.end);
    if start > end {
        return;
    }
    if start == end {
        if bounds.contains(&start) {
            rows.insert(text::point(text, start).row);
        }
        return;
    }
    let first = text::point(text, start).row;
    let last = text::point(text, end.saturating_sub(1)).row;
    rows.extend(first..=last);
}
