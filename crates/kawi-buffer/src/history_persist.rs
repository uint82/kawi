use std::collections::BTreeMap;

use ropey::Rope;

use crate::history::{History, Node};

mod reconstruct;
mod trim;

use reconstruct::{delta_between, reconstruct_texts, validate_entries};

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
    pub timestamp: u64,
    pub save_marker: Option<u64>,
    pub last_visited_child: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedHistory {
    pub current_id: u64,
    pub clean_id: Option<u64>,
    pub next_id: u64,
    pub next_write_marker: u64,
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
}

impl History {
    pub fn export(
        &self,
        current: &Rope,
        row: usize,
        col: usize,
        line_changes: u64,
    ) -> PersistedHistory {
        let context = ExportContext::new(self, current, row, col, line_changes);
        let entries = self
            .nodes
            .iter()
            .map(|(id, node)| export_entry(*id, node, &context))
            .collect();
        PersistedHistory {
            current_id: self.current_id,
            clean_id: self.clean_id,
            next_id: self.next_id,
            next_write_marker: self.next_write_marker,
            entries,
        }
    }

    pub fn from_persisted(current: &Rope, persisted: &PersistedHistory) -> Option<Self> {
        let entries = validate_entries(persisted)?;
        let texts = reconstruct_texts(current, persisted, &entries)?;
        let nodes = build_nodes(persisted, &texts)?;
        let max_id = persisted.entries.iter().map(|entry| entry.id).max()?;
        let max_marker = max_save_marker(persisted);
        let mut history = Self {
            nodes,
            current_id: persisted.current_id,
            next_id: persisted.next_id.max(max_id.saturating_add(1)),
            clean_id: persisted.clean_id,
            next_write_marker: persisted
                .next_write_marker
                .max(max_marker.saturating_add(1)),
            group_open: false,
        };
        history.evict_to_cap();
        Some(history)
    }
}

struct ExportContext<'a> {
    history: &'a History,
    current: &'a Rope,
    row: usize,
    col: usize,
    line_changes: u64,
    texts: BTreeMap<u64, &'a Rope>,
}

impl<'a> ExportContext<'a> {
    fn new(
        history: &'a History,
        current: &'a Rope,
        row: usize,
        col: usize,
        line_changes: u64,
    ) -> Self {
        let texts = history
            .nodes
            .iter()
            .map(|(id, node)| (*id, text_for(*id, node, history.current_id, current)))
            .collect();
        Self {
            history,
            current,
            row,
            col,
            line_changes,
            texts,
        }
    }

    fn text(&self, id: u64) -> &'a Rope {
        self.texts.get(&id).copied().unwrap_or(self.current)
    }

    fn row(&self, id: u64, node: &Node) -> usize {
        if id == self.history.current_id {
            self.row
        } else {
            node.row
        }
    }

    fn col(&self, id: u64, node: &Node) -> usize {
        if id == self.history.current_id {
            self.col
        } else {
            node.col
        }
    }

    fn line_changes(&self, id: u64, node: &Node) -> u64 {
        if id == self.history.current_id {
            self.line_changes
        } else {
            node.line_changes
        }
    }
}

fn text_for<'a>(id: u64, node: &'a Node, current_id: u64, current: &'a Rope) -> &'a Rope {
    if id == current_id {
        current
    } else {
        node.text.as_ref().unwrap_or(current)
    }
}

fn export_entry(id: u64, node: &Node, context: &ExportContext<'_>) -> HistoryEntry {
    let text = context.text(id);
    let (parent, delta) = match node.parent.and_then(|parent| context.texts.get(&parent)) {
        Some(parent_text) => (node.parent, delta_between(parent_text, text)),
        None => (None, HistoryDelta::default()),
    };
    HistoryEntry {
        id,
        parent,
        row: context.row(id, node),
        col: context.col(id, node),
        line_changes: context.line_changes(id, node),
        delta,
        timestamp: node.timestamp,
        save_marker: node.save_marker,
        last_visited_child: node.last_visited_child,
    }
}

fn build_nodes(
    persisted: &PersistedHistory,
    texts: &BTreeMap<u64, Rope>,
) -> Option<BTreeMap<u64, Node>> {
    persisted
        .entries
        .iter()
        .map(|entry| {
            Some((
                entry.id,
                Node {
                    parent: entry.parent,
                    text: Some(texts.get(&entry.id)?.clone()),
                    row: entry.row,
                    col: entry.col,
                    line_changes: entry.line_changes,
                    timestamp: entry.timestamp,
                    save_marker: entry.save_marker,
                    last_visited_child: entry.last_visited_child,
                },
            ))
        })
        .collect()
}

fn max_save_marker(persisted: &PersistedHistory) -> u64 {
    persisted
        .entries
        .iter()
        .filter_map(|entry| entry.save_marker)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
