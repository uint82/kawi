use std::collections::HashSet;

use super::history::{now_millis, History};

const MILLIS_PER_SECOND: u64 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoOffset {
    States(usize),
    Seconds(u64),
    FileWrites(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoLeaf {
    pub sequence: u64,
    pub age_millis: u64,
    pub changes: usize,
    pub save_marker: Option<u64>,
    pub current: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HistoryStep {
    Undo(u64),
    Redo(u64),
}

impl History {
    pub(crate) fn path_to(&self, target: u64) -> Option<Vec<HistoryStep>> {
        if !self.nodes.contains_key(&target) {
            return None;
        }
        if target == self.current_id {
            return Some(Vec::new());
        }
        let current_chain = self.ancestors_inclusive(self.current_id)?;
        let target_chain = self.ancestors_inclusive(target)?;
        let target_ids: HashSet<u64> = target_chain.iter().copied().collect();
        let lca = current_chain
            .iter()
            .copied()
            .find(|id| target_ids.contains(id))?;
        let mut steps = Vec::new();
        for id in current_chain.iter().copied().take_while(|id| *id != lca) {
            steps.push(HistoryStep::Undo(self.nodes.get(&id)?.parent?));
        }
        let mut down: Vec<u64> = target_chain
            .iter()
            .copied()
            .take_while(|id| *id != lca)
            .collect();
        down.reverse();
        steps.extend(down.into_iter().map(HistoryStep::Redo));
        Some(steps)
    }

    pub(crate) fn chronological_target(&self, forward: bool, count: usize) -> Option<u64> {
        if count == 0 {
            return Some(self.current_id);
        }
        if forward {
            self.nodes
                .range(self.current_id.saturating_add(1)..)
                .map(|(id, _)| *id)
                .take(count)
                .last()
        } else {
            self.nodes
                .range(..self.current_id)
                .map(|(id, _)| *id)
                .rev()
                .take(count)
                .last()
        }
    }

    pub(crate) fn offset_target(&self, offset: UndoOffset, forward: bool) -> Option<u64> {
        match offset {
            UndoOffset::States(count) => self.chronological_target(forward, count),
            UndoOffset::Seconds(seconds) => {
                self.time_target(forward, seconds.saturating_mul(MILLIS_PER_SECOND))
            }
            UndoOffset::FileWrites(count) => self.write_target(forward, count),
        }
    }

    pub fn leaves(&self) -> Vec<UndoLeaf> {
        self.leaves_at(now_millis())
    }

    pub(crate) fn leaves_at(&self, now: u64) -> Vec<UndoLeaf> {
        let parents: HashSet<u64> = self.nodes.values().filter_map(|node| node.parent).collect();
        self.nodes
            .iter()
            .filter(|(id, _)| !parents.contains(id))
            .filter_map(|(id, node)| {
                Some(UndoLeaf {
                    sequence: *id,
                    age_millis: if node.timestamp == 0 {
                        0
                    } else {
                        now.saturating_sub(node.timestamp)
                    },
                    changes: self.depth(*id)?,
                    save_marker: node.save_marker,
                    current: *id == self.current_id,
                })
            })
            .collect()
    }

    fn time_target(&self, forward: bool, delta: u64) -> Option<u64> {
        let current = self.nodes.get(&self.current_id)?;
        if forward {
            self.forward_time_target(current.timestamp.saturating_add(delta))
        } else {
            self.backward_time_target(current.timestamp.saturating_sub(delta))
        }
    }

    fn forward_time_target(&self, cutoff: u64) -> Option<u64> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.timestamp >= cutoff)
            .min_by_key(|(id, node)| (node.timestamp, **id))
            .map(|(id, _)| *id)
            .or_else(|| {
                self.nodes
                    .iter()
                    .max_by_key(|(id, node)| (node.timestamp, **id))
                    .map(|(id, _)| *id)
            })
    }

    fn backward_time_target(&self, cutoff: u64) -> Option<u64> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.timestamp <= cutoff)
            .max_by_key(|(id, node)| (node.timestamp, **id))
            .map(|(id, _)| *id)
            .or_else(|| {
                self.nodes
                    .iter()
                    .min_by_key(|(id, node)| (node.timestamp, **id))
                    .map(|(id, _)| *id)
            })
    }

    fn write_target(&self, forward: bool, count: usize) -> Option<u64> {
        if count == 0 {
            return Some(self.current_id);
        }
        let current = self.nodes.get(&self.current_id)?;
        let reference = current
            .save_marker
            .or_else(|| self.last_save_marker_before(self.current_id));
        if forward {
            self.forward_write_target(reference.unwrap_or(0), count)
        } else {
            let base = match current.save_marker {
                Some(marker) => marker,
                None => reference?.saturating_add(1),
            };
            self.backward_write_target(base, count)
        }
    }

    fn forward_write_target(&self, base: u64, count: usize) -> Option<u64> {
        let target = base.saturating_add(count as u64);
        self.nodes
            .iter()
            .filter(|(_, node)| node.save_marker.is_some_and(|marker| marker >= target))
            .min_by_key(|(id, node)| (node.save_marker.unwrap_or(u64::MAX), **id))
            .map(|(id, _)| *id)
            .or_else(|| self.nodes.keys().next_back().copied())
    }

    fn backward_write_target(&self, base: u64, count: usize) -> Option<u64> {
        let target = base.saturating_sub(count as u64);
        self.nodes
            .iter()
            .filter(|(_, node)| node.save_marker.is_some_and(|marker| marker <= target))
            .max_by_key(|(id, node)| (node.save_marker.unwrap_or(0), **id))
            .map(|(id, _)| *id)
            .or_else(|| self.nodes.keys().next().copied())
    }

    fn last_save_marker_before(&self, id: u64) -> Option<u64> {
        self.nodes
            .range(..id)
            .filter_map(|(_, node)| node.save_marker)
            .max()
    }

    fn depth(&self, id: u64) -> Option<usize> {
        Some(self.ancestors_inclusive(id)?.len().saturating_sub(1))
    }

    pub(crate) fn ancestors_inclusive(&self, id: u64) -> Option<Vec<u64>> {
        let mut chain = Vec::new();
        let mut seen = HashSet::new();
        let mut current = id;
        loop {
            if !seen.insert(current) {
                return None;
            }
            chain.push(current);
            let Some(parent) = self.nodes.get(&current)?.parent else {
                return Some(chain);
            };
            if !self.nodes.contains_key(&parent) {
                return None;
            }
            current = parent;
        }
    }
}

pub(crate) fn changed_cursor(before: &ropey::Rope, after: &ropey::Rope) -> Option<(usize, usize)> {
    let change = crate::diff::diff_ropes(before, after)?;
    if after.len_chars() == 0 {
        return Some((0, 0));
    }
    let index = change.start.min(after.len_chars().saturating_sub(1));
    let row = after.char_to_line(index);
    let col = index.saturating_sub(after.line_to_char(row));
    Some((row, col))
}
