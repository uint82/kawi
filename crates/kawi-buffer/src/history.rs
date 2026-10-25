pub(crate) const MAX_HISTORY: usize = 200;

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use ropey::Rope;

use crate::line_adjust::{self, LineAdjust};

pub(crate) struct Node {
    pub(crate) parent: Option<u64>,
    pub(crate) text: Option<Rope>,
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) line_changes: u64,
    pub(crate) timestamp: u64,
    pub(crate) save_marker: Option<u64>,
    pub(crate) last_visited_child: Option<u64>,
}

pub struct History {
    pub(crate) nodes: BTreeMap<u64, Node>,
    pub(crate) current_id: u64,
    pub(crate) next_id: u64,
    pub(crate) clean_id: Option<u64>,
    pub(crate) next_write_marker: u64,
    pub(crate) group_open: bool,
}

impl History {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            current_id: 0,
            next_id: 1,
            clean_id: Some(0),
            next_write_marker: 1,
            group_open: false,
        }
    }

    pub fn begin_change(&mut self) {
        self.group_open = true;
    }

    pub fn end_change(&mut self) {
        self.group_open = false;
    }

    pub fn checkpoint(&mut self, text: &Rope, row: usize, col: usize, line_changes: u64) {
        self.record_current(text, row, col, line_changes);
        let parent = self.current_id;
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.nodes.insert(
            id,
            Node {
                parent: Some(parent),
                text: None,
                row: 0,
                col: 0,
                line_changes,
                timestamp: now_millis(),
                save_marker: None,
                last_visited_child: None,
            },
        );
        if let Some(node) = self.nodes.get_mut(&parent) {
            node.last_visited_child = Some(id);
        }
        self.current_id = id;
        self.group_open = false;
        self.evict_to_cap();
    }

    pub fn maybe_checkpoint(&mut self, text: &Rope, row: usize, col: usize, line_changes: u64) {
        if self.group_open {
            self.checkpoint(text, row, col, line_changes);
        }
    }

    pub fn undo(
        &mut self,
        text: &mut Rope,
        row: usize,
        col: usize,
        line_changes: u64,
    ) -> Option<((usize, usize), Option<LineAdjust>)> {
        let parent = self.nodes.get(&self.current_id)?.parent?;
        self.undo_to(text, row, col, line_changes, parent)
    }

    pub fn redo(
        &mut self,
        text: &mut Rope,
        row: usize,
        col: usize,
        line_changes: u64,
    ) -> Option<((usize, usize), Option<LineAdjust>)> {
        let child = self.redo_child(self.current_id)?;
        self.redo_to(text, row, col, line_changes, child)
    }

    pub(crate) fn undo_to(
        &mut self,
        text: &mut Rope,
        row: usize,
        col: usize,
        line_changes: u64,
        target: u64,
    ) -> Option<((usize, usize), Option<LineAdjust>)> {
        self.record_current(text, row, col, line_changes);
        let current = self.current_id;
        let parent = self.nodes.get(&current)?.parent?;
        if parent != target {
            return None;
        }
        if let Some(node) = self.nodes.get_mut(&parent) {
            node.last_visited_child = Some(current);
        }
        self.move_to(text, target, line_changes)
    }

    pub(crate) fn redo_to(
        &mut self,
        text: &mut Rope,
        row: usize,
        col: usize,
        line_changes: u64,
        target: u64,
    ) -> Option<((usize, usize), Option<LineAdjust>)> {
        self.record_current(text, row, col, line_changes);
        let current = self.current_id;
        if self.nodes.get(&target)?.parent != Some(current) {
            return None;
        }
        if let Some(node) = self.nodes.get_mut(&current) {
            node.last_visited_child = Some(target);
        }
        self.move_to(text, target, line_changes)
    }

    fn move_to(
        &mut self,
        text: &mut Rope,
        target: u64,
        line_changes: u64,
    ) -> Option<((usize, usize), Option<LineAdjust>)> {
        let (target_text, target_row, target_col, adjust) =
            self.target_state(text, target, line_changes)?;
        self.current_id = target;
        *text = target_text;
        Some(((target_row, target_col), adjust))
    }

    fn target_state(
        &self,
        text: &Rope,
        target: u64,
        line_changes: u64,
    ) -> Option<(Rope, usize, usize, Option<LineAdjust>)> {
        let node = self.nodes.get(&target)?;
        let target_text = node.text.clone()?;
        let adjust = adjust_between(text, &target_text, node.line_changes, line_changes);
        Some((target_text, node.row, node.col, adjust))
    }

    pub fn mark_clean(&mut self) {
        self.clean_id = Some(self.current_id);
        let marker = self.next_write_marker;
        self.next_write_marker = self.next_write_marker.saturating_add(1);
        if let Some(node) = self.nodes.get_mut(&self.current_id) {
            node.save_marker = Some(marker);
        }
    }

    pub(crate) fn mark_clean_at(&mut self, text: &Rope, line_changes: u64) {
        if let Some(node) = self.nodes.get_mut(&self.current_id) {
            if node.text.is_none() {
                node.text = Some(text.clone());
                node.line_changes = line_changes;
            }
        } else {
            self.nodes.insert(
                self.current_id,
                Node {
                    parent: None,
                    text: Some(text.clone()),
                    row: 0,
                    col: 0,
                    line_changes,
                    timestamp: now_millis(),
                    save_marker: None,
                    last_visited_child: None,
                },
            );
        }
        self.mark_clean();
    }

    pub fn is_dirty(&self) -> bool {
        self.clean_id != Some(self.current_id)
    }

    pub fn current_id(&self) -> u64 {
        self.current_id
    }

    fn record_current(&mut self, text: &Rope, row: usize, col: usize, line_changes: u64) {
        if let Some(node) = self.nodes.get_mut(&self.current_id) {
            node.text = Some(text.clone());
            node.row = row;
            node.col = col;
            node.line_changes = line_changes;
            return;
        }
        self.nodes.insert(
            self.current_id,
            Node {
                parent: None,
                text: Some(text.clone()),
                row,
                col,
                line_changes,
                timestamp: now_millis(),
                save_marker: None,
                last_visited_child: None,
            },
        );
    }

    fn redo_child(&self, parent: u64) -> Option<u64> {
        let node = self.nodes.get(&parent)?;
        if let Some(child) = node.last_visited_child {
            if self.nodes.get(&child)?.parent == Some(parent) {
                return Some(child);
            }
        }
        self.nodes
            .iter()
            .filter(|(_, child)| child.parent == Some(parent))
            .map(|(id, _)| *id)
            .next_back()
    }
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

fn adjust_between(
    text: &Rope,
    target: &Rope,
    target_line_changes: u64,
    line_changes: u64,
) -> Option<LineAdjust> {
    if target_line_changes == line_changes {
        return None;
    }
    line_adjust::undo(text, target)
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod evict_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
