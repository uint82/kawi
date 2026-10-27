mod indent;
mod visible;

use std::collections::BTreeSet;

use kawi_buffer::{Buffer, LineAdjust};

pub const DEFAULT_FOLD_COLUMN: usize = 1;
pub const MAX_FOLD_COLUMN: usize = 1;

const DEFAULT_SHIFTWIDTH: usize = 8;
const DEFAULT_TABSTOP: usize = 8;
const MANUAL_FOLD_DEPTH: usize = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FoldMethod {
    #[default]
    Manual,
    Indent,
}

impl FoldMethod {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "manual" => Some(Self::Manual),
            "indent" => Some(Self::Indent),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Indent => "indent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fold {
    pub start: usize,
    pub end: usize,
    pub depth: usize,
}

impl Fold {
    fn new(start: usize, end: usize, depth: usize) -> Self {
        Self { start, end, depth }
    }

    fn contains(self, row: usize) -> bool {
        self.start <= row && row <= self.end
    }
}

#[derive(Debug)]
pub struct FoldState {
    method: FoldMethod,
    manual: Vec<Fold>,
    indent: Vec<Fold>,
    indent_revision: Option<u64>,
    closed: BTreeSet<usize>,
    shiftwidth: usize,
    tabstop: usize,
    column: usize,
}

impl Default for FoldState {
    fn default() -> Self {
        Self {
            method: FoldMethod::default(),
            manual: Vec::new(),
            indent: Vec::new(),
            indent_revision: None,
            closed: BTreeSet::new(),
            shiftwidth: DEFAULT_SHIFTWIDTH,
            tabstop: DEFAULT_TABSTOP,
            column: DEFAULT_FOLD_COLUMN,
        }
    }
}

impl FoldState {
    pub fn method(&self) -> FoldMethod {
        self.method
    }

    pub fn set_method(&mut self, method: FoldMethod) {
        if self.method == method {
            return;
        }
        self.method = method;
        self.retain_closed();
    }

    pub fn set_column(&mut self, column: usize) {
        self.column = column;
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn set_indent(&mut self, shiftwidth: usize, tabstop: usize) {
        let shiftwidth = shiftwidth.max(1);
        let tabstop = tabstop.max(1);
        if self.shiftwidth == shiftwidth && self.tabstop == tabstop {
            return;
        }
        self.shiftwidth = shiftwidth;
        self.tabstop = tabstop;
        self.indent_revision = None;
    }

    pub fn refresh(&mut self, buffer: &Buffer) {
        if self.method != FoldMethod::Indent || self.indent_revision == Some(buffer.revision()) {
            return;
        }
        self.indent = indent::folds(buffer, self.shiftwidth, self.tabstop);
        self.indent_revision = Some(buffer.revision());
        self.retain_closed();
    }

    pub fn folds(&self) -> &[Fold] {
        match self.method {
            FoldMethod::Manual => &self.manual,
            FoldMethod::Indent => &self.indent,
        }
    }

    pub fn has_folds(&self) -> bool {
        !self.folds().is_empty()
    }

    pub fn column_width(&self) -> usize {
        if self.column == 0 || !self.has_folds() {
            0
        } else {
            self.column.min(MAX_FOLD_COLUMN)
        }
    }

    pub fn is_closed(&self, start: usize) -> bool {
        self.closed.contains(&start)
    }

    pub fn fold_at(&self, row: usize) -> Option<Fold> {
        self.folds()
            .iter()
            .copied()
            .filter(|fold| fold.contains(row))
            .max_by_key(|fold| (fold.start, fold.end))
    }

    pub fn marker(&self, row: usize) -> char {
        if let Some(fold) = self.folds().iter().find(|fold| fold.start == row) {
            return if self.is_closed(fold.start) { '+' } else { '-' };
        }
        let open = self
            .folds()
            .iter()
            .any(|fold| fold.start < row && row <= fold.end && !self.is_closed(fold.start));
        if open {
            '|'
        } else {
            ' '
        }
    }

    pub fn toggle_at(&mut self, row: usize) -> Option<usize> {
        let fold = self.fold_at(row)?;
        if self.is_closed(fold.start) {
            self.closed.remove(&fold.start);
            None
        } else {
            self.closed.insert(fold.start);
            (row > fold.start).then_some(fold.start)
        }
    }

    pub fn open_at(&mut self, row: usize) -> Option<usize> {
        let fold = self.fold_at(row)?;
        self.closed.remove(&fold.start);
        None
    }

    pub fn close_at(&mut self, row: usize) -> Option<usize> {
        let fold = self.fold_at(row)?;
        self.closed.insert(fold.start);
        (row > fold.start).then_some(fold.start)
    }

    pub fn open_all(&mut self) {
        self.closed.clear();
    }

    pub fn close_all(&mut self) {
        self.closed = self.folds().iter().map(|fold| fold.start).collect();
    }

    pub fn delete_at(&mut self, row: usize) -> Option<usize> {
        if self.method != FoldMethod::Manual {
            return None;
        }
        let fold = self.fold_at(row)?;
        self.manual
            .retain(|candidate| candidate.start != fold.start || candidate.end != fold.end);
        self.closed.remove(&fold.start);
        (row > fold.start).then_some(fold.start)
    }

    pub fn create_manual(&mut self, start: usize, end: usize, line_count: usize) {
        if start >= end || end >= line_count {
            return;
        }
        self.manual
            .retain(|fold| fold.start != start || fold.end != end);
        self.manual.push(Fold::new(start, end, MANUAL_FOLD_DEPTH));
        self.manual.sort_by_key(|fold| (fold.start, fold.end));
        self.closed.insert(start);
    }

    pub fn adjust(&mut self, adjust: LineAdjust) {
        self.manual = self
            .manual
            .iter()
            .filter_map(|fold| {
                let start = adjusted_row(fold.start, adjust)?;
                let end = adjusted_row(fold.end, adjust)?;
                (end > start).then(|| Fold::new(start, end, fold.depth))
            })
            .collect();
        self.manual.sort_by_key(|fold| (fold.start, fold.end));
        self.manual.dedup_by_key(|fold| (fold.start, fold.end));
        self.closed = self
            .closed
            .iter()
            .filter_map(|start| adjusted_row(*start, adjust))
            .collect();
        if self.method == FoldMethod::Manual {
            self.retain_closed();
        }
    }

    fn retain_closed(&mut self) {
        let starts: BTreeSet<usize> = self.folds().iter().map(|fold| fold.start).collect();
        self.closed.retain(|start| starts.contains(start));
    }
}

fn adjusted_row(row: usize, adjust: LineAdjust) -> Option<usize> {
    let line = row.saturating_add(1);
    let moved = if line >= adjust.line1 && line <= adjust.line2 {
        match adjust.amount {
            Some(amount) => line.saturating_add_signed(amount),
            None => adjust.line1,
        }
    } else if adjust.amount_after != 0 && line > adjust.line2 {
        line.saturating_add_signed(adjust.amount_after)
    } else {
        line
    };
    moved.checked_sub(1)
}

#[cfg(test)]
mod tests;
