use std::collections::BTreeSet;

use super::{Fold, FoldMethod, FoldState};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldSnapshot {
    pub manual: Vec<Fold>,
    pub closed: Vec<usize>,
}

impl FoldSnapshot {
    pub fn is_empty(&self) -> bool {
        self.manual.is_empty() && self.closed.is_empty()
    }
}

impl FoldState {
    pub fn snapshot(&self) -> FoldSnapshot {
        FoldSnapshot {
            manual: self.manual.clone(),
            closed: self.closed.iter().copied().collect(),
        }
    }

    pub fn restore(&mut self, snapshot: FoldSnapshot, line_count: usize) {
        self.manual = normalize_manual(snapshot.manual, line_count);
        self.closed = normalize_closed(snapshot.closed, line_count);
        self.retain_closed_if_ready();
    }

    pub fn drop_manual(&mut self) {
        self.manual.clear();
        if self.method == FoldMethod::Manual {
            self.closed.clear();
        }
    }

    pub fn revalidate(&mut self, line_count: usize) {
        self.manual = normalize_manual(std::mem::take(&mut self.manual), line_count);
        self.closed = normalize_closed(std::mem::take(&mut self.closed), line_count);
        self.retain_closed_if_ready();
    }

    fn retain_closed_if_ready(&mut self) {
        if self.method == FoldMethod::Manual || self.indent_revision.is_some() {
            self.retain_closed();
        }
    }
}

fn normalize_manual(folds: Vec<Fold>, line_count: usize) -> Vec<Fold> {
    let mut folds: Vec<Fold> = folds
        .into_iter()
        .filter(|fold| fold.start < fold.end && fold.end < line_count)
        .map(|fold| Fold::manual(fold.start, fold.end))
        .collect();
    folds.sort_by_key(|fold| (fold.start, fold.end));
    folds.dedup_by_key(|fold| (fold.start, fold.end));
    folds
}

fn normalize_closed(starts: impl IntoIterator<Item = usize>, line_count: usize) -> BTreeSet<usize> {
    starts
        .into_iter()
        .filter(|start| *start < line_count)
        .collect()
}
