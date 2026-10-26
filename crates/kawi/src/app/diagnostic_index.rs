use super::problems::{Problem, ProblemSeverity};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const LSP_SOURCE_PREFIX: &str = "lsp:";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct DiagnosticCounts {
    pub(super) errors: usize,
    pub(super) warnings: usize,
    pub(super) information: usize,
    pub(super) hints: usize,
}

impl DiagnosticCounts {
    pub(super) fn add(&mut self, severity: ProblemSeverity) {
        match severity {
            ProblemSeverity::Error => self.errors = self.errors.saturating_add(1),
            ProblemSeverity::Warning => self.warnings = self.warnings.saturating_add(1),
            ProblemSeverity::Information => {
                self.information = self.information.saturating_add(1);
            }
            ProblemSeverity::Hint => self.hints = self.hints.saturating_add(1),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DiagnosticNavigation {
    pub(super) row: usize,
    pub(super) column: usize,
    pub(super) index: usize,
    pub(super) total: usize,
    pub(super) severity: ProblemSeverity,
    pub(super) message: String,
}

#[derive(Default)]
pub(super) struct DiagnosticIndex {
    by_path: BTreeMap<PathBuf, BTreeMap<usize, Vec<Problem>>>,
    buffer_counts: BTreeMap<PathBuf, DiagnosticCounts>,
    workspace_counts: DiagnosticCounts,
}

impl DiagnosticIndex {
    pub(super) fn build<'a>(
        diagnostics: impl Iterator<Item = (&'a Path, &'a str, &'a Problem)>,
        source_ranks: &BTreeMap<String, usize>,
        stale_sources: &BTreeSet<String>,
    ) -> Self {
        let mut index = Self::default();
        for (path, source, problem) in diagnostics {
            if stale_sources.contains(source) {
                continue;
            }
            let line = problem.line.saturating_sub(1);
            index
                .by_path
                .entry(path.to_path_buf())
                .or_default()
                .entry(line)
                .or_default()
                .push(problem.clone());
            if source.starts_with(LSP_SOURCE_PREFIX) {
                index
                    .buffer_counts
                    .entry(path.to_path_buf())
                    .or_default()
                    .add(problem.severity);
                index.workspace_counts.add(problem.severity);
            }
        }
        index.sort_lines(source_ranks);
        index
    }

    pub(super) fn at(&self, path: &Path, line: usize) -> &[Problem] {
        self.by_path
            .get(path)
            .and_then(|lines| lines.get(&line))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(super) fn primary(
        &self,
        path: &Path,
        line: usize,
        column: Option<usize>,
    ) -> Option<&Problem> {
        let diagnostics = self.at(path, line);
        column
            .and_then(|column| {
                diagnostics
                    .iter()
                    .find(|problem| overlaps(problem, line, column))
            })
            .or_else(|| diagnostics.first())
    }

    pub(super) fn counts(&self, path: &Path) -> DiagnosticCounts {
        self.buffer_counts.get(path).copied().unwrap_or_default()
    }

    pub(super) fn workspace_counts(&self) -> DiagnosticCounts {
        self.workspace_counts
    }

    pub(super) fn navigate_lsp(
        &self,
        path: &Path,
        line: usize,
        column: usize,
        forward: bool,
    ) -> Option<DiagnosticNavigation> {
        let mut diagnostics: Vec<&Problem> = self
            .by_path
            .get(path)?
            .values()
            .flatten()
            .filter(|problem| problem.source.starts_with(LSP_SOURCE_PREFIX))
            .collect();
        if diagnostics.is_empty() {
            return None;
        }
        diagnostics.sort_by_key(|problem| (problem.line, problem.column, problem.severity));
        let position = (line.saturating_add(1), column.saturating_add(1));
        let index = if forward {
            diagnostics
                .iter()
                .position(|problem| (problem.line, problem.column) > position)
                .unwrap_or(0)
        } else {
            diagnostics
                .iter()
                .rposition(|problem| (problem.line, problem.column) < position)
                .unwrap_or(diagnostics.len().saturating_sub(1))
        };
        let problem = diagnostics[index];
        Some(DiagnosticNavigation {
            row: problem.line.saturating_sub(1),
            column: problem.column.saturating_sub(1),
            index: index.saturating_add(1),
            total: diagnostics.len(),
            severity: problem.severity,
            message: problem.message.clone(),
        })
    }

    fn sort_lines(&mut self, source_ranks: &BTreeMap<String, usize>) {
        for lines in self.by_path.values_mut() {
            for diagnostics in lines.values_mut() {
                diagnostics.sort_by(|left, right| {
                    left.severity
                        .cmp(&right.severity)
                        .then_with(|| left.column.cmp(&right.column))
                        .then_with(|| {
                            source_rank(left, source_ranks).cmp(&source_rank(right, source_ranks))
                        })
                        .then_with(|| left.message.cmp(&right.message))
                });
            }
        }
    }
}

fn source_rank(problem: &Problem, source_ranks: &BTreeMap<String, usize>) -> usize {
    source_ranks
        .get(&problem.source)
        .copied()
        .unwrap_or(usize::MAX)
}

fn overlaps(problem: &Problem, line: usize, column: usize) -> bool {
    let line = line.saturating_add(1);
    let column = column.saturating_add(1);
    if line < problem.line || line > problem.end_line {
        return false;
    }
    if line == problem.line && column < problem.column {
        return false;
    }
    if line == problem.end_line {
        let zero_width = problem.line == problem.end_line && problem.column == problem.end_column;
        let end = if zero_width {
            problem.end_column.saturating_add(1)
        } else {
            problem.end_column
        };
        if column >= end {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests;
