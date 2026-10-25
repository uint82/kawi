use kawi_buffer::{diff_ropes, RopeChange};
use kawi_lsp::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag};
use kawi_task as task;
use ropey::Rope;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::file_ops::{path_under, rewrite_path};

const MAX_PROBLEMS: usize = 10_000;
const TASK_PREFIX: &str = "task:";
const LSP_PREFIX: &str = "lsp:";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ProblemSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl ProblemSeverity {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Information => "info",
            Self::Hint => "hint",
        }
    }
}

impl From<task::Severity> for ProblemSeverity {
    fn from(severity: task::Severity) -> Self {
        match severity {
            task::Severity::Error => Self::Error,
            task::Severity::Warning => Self::Warning,
            task::Severity::Information => Self::Information,
            task::Severity::Hint => Self::Hint,
        }
    }
}

impl From<DiagnosticSeverity> for ProblemSeverity {
    fn from(severity: DiagnosticSeverity) -> Self {
        match severity {
            DiagnosticSeverity::Error => Self::Error,
            DiagnosticSeverity::Warning => Self::Warning,
            DiagnosticSeverity::Information => Self::Information,
            DiagnosticSeverity::Hint => Self::Hint,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProblemTag {
    Unnecessary,
    Deprecated,
}

impl ProblemTag {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Unnecessary => "unused",
            Self::Deprecated => "deprecated",
        }
    }
}

impl From<DiagnosticTag> for ProblemTag {
    fn from(tag: DiagnosticTag) -> Self {
        match tag {
            DiagnosticTag::Unnecessary => Self::Unnecessary,
            DiagnosticTag::Deprecated => Self::Deprecated,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RelatedProblem {
    pub(super) path: PathBuf,
    pub(super) line: usize,
    pub(super) message: String,
}

impl From<DiagnosticRelatedInformation> for RelatedProblem {
    fn from(information: DiagnosticRelatedInformation) -> Self {
        Self {
            path: information.path,
            line: usize::try_from(information.line)
                .unwrap_or(usize::MAX)
                .saturating_add(1),
            message: information.message,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Problem {
    pub(super) source: String,
    pub(super) path: PathBuf,
    pub(super) line: usize,
    pub(super) column: usize,
    pub(super) end_line: usize,
    pub(super) end_column: usize,
    pub(super) severity: ProblemSeverity,
    pub(super) message: String,
    pub(super) code: Option<String>,
    pub(super) provider: Option<String>,
    pub(super) tags: Vec<ProblemTag>,
    pub(super) related: Vec<RelatedProblem>,
}

impl Problem {
    fn from_task(source: String, finding: task::Finding) -> Self {
        Self {
            source,
            path: finding.path,
            line: finding.line,
            column: finding.column,
            end_line: finding.line,
            end_column: finding.column,
            severity: finding.severity.into(),
            message: finding.message,
            code: None,
            provider: None,
            tags: Vec::new(),
            related: Vec::new(),
        }
    }

    fn from_lsp(source: String, path: &Path, diagnostic: Diagnostic) -> Self {
        let line = usize::try_from(diagnostic.range.start.line)
            .unwrap_or(usize::MAX)
            .saturating_add(1);
        let column = usize::try_from(diagnostic.range.start.character)
            .unwrap_or(usize::MAX)
            .saturating_add(1);
        let end_line = usize::try_from(diagnostic.range.end.line)
            .unwrap_or(usize::MAX)
            .saturating_add(1);
        let end_column = usize::try_from(diagnostic.range.end.character)
            .unwrap_or(usize::MAX)
            .saturating_add(1);
        Self {
            source,
            path: path.to_path_buf(),
            line,
            column,
            end_line,
            end_column,
            severity: diagnostic.severity.into(),
            message: compact_message(&diagnostic.message),
            code: diagnostic.code,
            provider: diagnostic.source,
            tags: diagnostic.tags.into_iter().map(ProblemTag::from).collect(),
            related: diagnostic
                .related
                .into_iter()
                .map(RelatedProblem::from)
                .collect(),
        }
    }

    fn key(&self) -> ProblemKey {
        ProblemKey {
            source: self.source.clone(),
            path: self.path.clone(),
            line: self.line,
            column: self.column,
            end_line: self.end_line,
            end_column: self.end_column,
            message: self.message.clone(),
            code: self.code.clone(),
        }
    }

    fn provider_label(&self) -> &str {
        self.provider
            .as_deref()
            .filter(|provider| !provider.is_empty())
            .unwrap_or_else(|| self.source.strip_prefix(LSP_PREFIX).unwrap_or(&self.source))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ProblemKey {
    source: String,
    path: PathBuf,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
    message: String,
    code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    source: String,
    path: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum SeverityFilter {
    #[default]
    All,
    Error,
    Warning,
    Information,
    Hint,
}

impl SeverityFilter {
    fn next(self) -> Self {
        match self {
            Self::All => Self::Error,
            Self::Error => Self::Warning,
            Self::Warning => Self::Information,
            Self::Information => Self::Hint,
            Self::Hint => Self::All,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Error => "errors",
            Self::Warning => "warnings",
            Self::Information => "info",
            Self::Hint => "hints",
        }
    }

    fn matches(self, severity: ProblemSeverity) -> bool {
        match self {
            Self::All => true,
            Self::Error => severity == ProblemSeverity::Error,
            Self::Warning => severity == ProblemSeverity::Warning,
            Self::Information => severity == ProblemSeverity::Information,
            Self::Hint => severity == ProblemSeverity::Hint,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct ProblemFilter {
    severity: SeverityFilter,
    current_file_only: bool,
}

impl ProblemFilter {
    pub(super) fn severity_label(self) -> &'static str {
        self.severity.label()
    }

    pub(super) fn current_file_only(self) -> bool {
        self.current_file_only
    }

    fn matches(self, problem: &Problem, current_path: Option<&Path>) -> bool {
        if !self.severity.matches(problem.severity) {
            return false;
        }
        !self.current_file_only || current_path.is_some_and(|path| problem.path == path)
    }
}

struct EditorText {
    path: PathBuf,
    revision: u64,
    rope: Rope,
}

#[derive(Default)]
pub(super) struct Problems {
    groups: BTreeMap<GroupKey, Vec<Problem>>,
    filter: ProblemFilter,
    current: Option<ProblemKey>,
    editor: Option<EditorText>,
    stale_sources: std::collections::BTreeSet<String>,
}

impl Problems {
    pub(super) fn clear_source(&mut self, source: &str) {
        self.stale_sources.remove(source);
        self.groups.retain(|key, _| key.source != source);
    }

    pub(super) fn rewrite_paths(&mut self, old: &Path, new: &Path) {
        let groups = std::mem::take(&mut self.groups);
        for (mut key, problems) in groups {
            let Some(path) = rewrite_path(&key.path, old, new) else {
                self.groups.insert(key, problems);
                continue;
            };
            key.path = path;
            let problems = problems
                .into_iter()
                .map(|mut problem| {
                    problem.path = rewrite_path(&problem.path, old, new)
                        .unwrap_or_else(|| problem.path.clone());
                    for related in &mut problem.related {
                        if let Some(path) = rewrite_path(&related.path, old, new) {
                            related.path = path;
                        }
                    }
                    problem
                })
                .collect::<Vec<_>>();
            self.groups.entry(key).or_default().extend(problems);
        }
        if let Some(current) = self.current.as_mut() {
            if let Some(path) = rewrite_path(&current.path, old, new) {
                current.path = path;
            }
        }
        if let Some(editor) = self.editor.as_mut() {
            if let Some(path) = rewrite_path(&editor.path, old, new) {
                editor.path = path;
            }
        }
    }

    pub(super) fn remove_paths(&mut self, target: &Path) {
        self.groups.retain(|key, _| !path_under(&key.path, target));
        if self
            .current
            .as_ref()
            .is_some_and(|current| path_under(&current.path, target))
        {
            self.current = None;
        }
        if self
            .editor
            .as_ref()
            .is_some_and(|editor| path_under(&editor.path, target))
        {
            self.editor = None;
        }
    }

    pub(super) fn mark_tasks_stale(&mut self) {
        for key in self.groups.keys() {
            if key.source.starts_with(TASK_PREFIX) {
                self.stale_sources.insert(key.source.clone());
            }
        }
    }

    pub(super) fn is_stale(&self, source: &str) -> bool {
        self.stale_sources.contains(source)
    }

    pub(super) fn clear_lsp_source(&mut self, server: &str) {
        self.clear_source(&lsp_source(server));
    }

    pub(super) fn replace_lsp(&mut self, server: &str, path: &Path, diagnostics: Vec<Diagnostic>) {
        let key = GroupKey {
            source: lsp_source(server),
            path: path.to_path_buf(),
        };
        let problems = diagnostics
            .into_iter()
            .map(|diagnostic| Problem::from_lsp(key.source.clone(), path, diagnostic))
            .collect();
        self.replace_group(key, problems);
    }

    pub(super) fn push_task(&mut self, task_name: &str, finding: task::Finding) -> bool {
        let source = task_source(task_name);
        self.stale_sources.remove(&source);
        let key = GroupKey {
            source: source.clone(),
            path: finding.path.clone(),
        };
        self.push(key, Problem::from_task(source, finding))
    }

    pub(super) fn cycle_severity(&mut self) {
        self.filter.severity = self.filter.severity.next();
    }

    pub(super) fn toggle_current_file(&mut self) {
        self.filter.current_file_only = !self.filter.current_file_only;
    }

    pub(super) fn filter(&self) -> ProblemFilter {
        self.filter
    }

    pub(super) fn visible(&self, current_path: Option<&Path>) -> Vec<&Problem> {
        let mut visible: Vec<&Problem> = self
            .groups
            .values()
            .flatten()
            .filter(|problem| self.filter.matches(problem, current_path))
            .collect();
        sort_problems(&mut visible);
        visible
    }

    pub(super) fn total(&self) -> usize {
        self.groups.values().map(Vec::len).sum()
    }

    pub(super) fn source_count(&self) -> usize {
        self.groups
            .keys()
            .map(|key| &key.source)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    }

    pub(super) fn advance(
        &mut self,
        current_path: Option<&Path>,
        forward: bool,
    ) -> Option<(usize, usize, Problem)> {
        let (index, total, problem) = {
            let visible = self.visible(current_path);
            if visible.is_empty() {
                return None;
            }
            let index = navigation_index(&visible, self.current.as_ref(), forward);
            (index, visible.len(), visible[index].clone())
        };
        self.current = Some(problem.key());
        Some((index, total, problem))
    }

    pub(super) fn observe_editor(&mut self, path: Option<&Path>, revision: u64, rope: Rope) {
        let Some(path) = path else {
            self.editor = None;
            return;
        };
        if self
            .editor
            .as_ref()
            .is_some_and(|editor| editor.path == path && editor.revision == revision)
        {
            return;
        }
        let previous = self.editor.replace(EditorText {
            path: path.to_path_buf(),
            revision,
            rope,
        });
        let Some(previous) = previous else {
            return;
        };
        let Some(current) = self.editor.as_ref() else {
            return;
        };
        if previous.path != current.path || previous.revision == current.revision {
            return;
        }
        let Some(change) = diff_ropes(&previous.rope, &current.rope) else {
            return;
        };
        let path = current.path.clone();
        let old = previous.rope;
        let new = current.rope.clone();
        self.shift_lsp(&path, &old, &new, change);
    }

    fn shift_lsp(&mut self, path: &Path, old: &Rope, new: &Rope, change: RopeChange) {
        for (key, problems) in &mut self.groups {
            if key.path != path || !key.source.starts_with(LSP_PREFIX) {
                continue;
            }
            for problem in problems.iter_mut() {
                shift_problem(problem, old, new, change);
            }
        }
    }

    fn push(&mut self, key: GroupKey, problem: Problem) -> bool {
        if self.total() >= MAX_PROBLEMS {
            return false;
        }
        self.groups.entry(key).or_default().push(problem);
        true
    }

    fn replace_group(&mut self, key: GroupKey, problems: Vec<Problem>) {
        let others = self
            .total()
            .saturating_sub(self.groups.get(&key).map_or(0, Vec::len));
        let capacity = MAX_PROBLEMS.saturating_sub(others);
        if problems.is_empty() || capacity == 0 {
            self.groups.remove(&key);
            return;
        }
        let mut problems = problems;
        problems.truncate(capacity);
        self.groups.insert(key, problems);
    }
}

pub(super) fn task_source(task_name: &str) -> String {
    format!("{TASK_PREFIX}{task_name}")
}

fn lsp_source(server: &str) -> String {
    format!("{LSP_PREFIX}{server}")
}

pub(super) fn source_label(source: &str) -> &str {
    source
}

pub(super) fn code_label(problem: &Problem) -> Option<String> {
    let code = problem.code.as_deref()?;
    Some(format!("{} {code}", problem.provider_label()))
}

fn sort_problems(problems: &mut [&Problem]) {
    problems.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column))
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.message.cmp(&right.message))
    });
}

fn navigation_index(visible: &[&Problem], current: Option<&ProblemKey>, forward: bool) -> usize {
    let Some(current) = current else {
        return if forward {
            0
        } else {
            visible.len().saturating_sub(1)
        };
    };
    if let Some(index) = visible.iter().position(|problem| problem.key() == *current) {
        return if forward {
            index.saturating_add(1) % visible.len()
        } else {
            index
                .checked_sub(1)
                .unwrap_or(visible.len().saturating_sub(1))
        };
    }
    if forward {
        visible
            .iter()
            .position(|problem| problem.key() > *current)
            .unwrap_or(0)
    } else {
        visible
            .iter()
            .rposition(|problem| problem.key() < *current)
            .unwrap_or(visible.len().saturating_sub(1))
    }
}

fn shift_problem(problem: &mut Problem, old: &Rope, new: &Rope, change: RopeChange) {
    let start = offset_for(old, problem.line, problem.column);
    let end = offset_for(old, problem.end_line, problem.end_column).max(start);
    let start = remap_offset(start, change, new.len_chars());
    let end = remap_offset(end, change, new.len_chars()).max(start);
    let (line, column) = line_column(new, start);
    let (end_line, end_column) = line_column(new, end);
    problem.line = line;
    problem.column = column;
    problem.end_line = end_line;
    problem.end_column = end_column;
}

fn remap_offset(offset: usize, change: RopeChange, new_len: usize) -> usize {
    let remapped = if offset < change.start {
        offset
    } else if offset >= change.old_end {
        offset
            .saturating_sub(change.old_end)
            .saturating_add(change.new_end)
    } else {
        change.new_end
    };
    remapped.min(new_len)
}

fn offset_for(rope: &Rope, line: usize, column: usize) -> usize {
    let line = line
        .saturating_sub(1)
        .min(rope.len_lines().saturating_sub(1));
    let start = rope.line_to_char(line);
    start.saturating_add(column.saturating_sub(1).min(line_len(rope, line)))
}

fn line_column(rope: &Rope, offset: usize) -> (usize, usize) {
    let offset = offset.min(rope.len_chars());
    let line = rope.char_to_line(offset);
    let start = rope.line_to_char(line);
    (
        line.saturating_add(1),
        offset.saturating_sub(start).saturating_add(1),
    )
}

fn line_len(rope: &Rope, line: usize) -> usize {
    let start = rope.line_to_char(line);
    let mut end = if line.saturating_add(1) < rope.len_lines() {
        rope.line_to_char(line.saturating_add(1))
    } else {
        rope.len_chars()
    };
    while end > start && matches!(rope.char(end.saturating_sub(1)), '\n' | '\r') {
        end = end.saturating_sub(1);
    }
    end.saturating_sub(start)
}

fn compact_message(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_lsp::{DiagnosticPosition, DiagnosticRange, DiagnosticRelatedInformation};
    use kawi_task::Severity;

    fn lsp_diagnostic(line: u32, message: &str, severity: DiagnosticSeverity) -> Diagnostic {
        Diagnostic {
            range: DiagnosticRange {
                start: DiagnosticPosition { line, character: 0 },
                end: DiagnosticPosition { line, character: 4 },
            },
            severity,
            message: message.to_string(),
            source: Some("rustc".to_string()),
            code: Some("E0308".to_string()),
            tags: Vec::new(),
            related: Vec::new(),
        }
    }

    fn task_finding(line: usize) -> task::Finding {
        task::Finding {
            path: PathBuf::from("src/main.rs"),
            line,
            column: 1,
            severity: Severity::Error,
            message: format!("task {line}"),
        }
    }

    #[test]
    fn sources_are_replaced_independently() {
        let path = Path::new("src/main.rs");
        let mut problems = Problems::default();
        problems.push_task("check", task_finding(1));
        problems.replace_lsp(
            "rust-analyzer",
            path,
            vec![lsp_diagnostic(2, "lsp", DiagnosticSeverity::Error)],
        );
        assert_eq!(problems.total(), 2);
        problems.replace_lsp("rust-analyzer", path, Vec::new());
        assert_eq!(problems.total(), 1);
        assert_eq!(problems.visible(Some(path))[0].source, "task:check");
        problems.clear_source("task:check");
        assert_eq!(problems.total(), 0);
    }

    #[test]
    fn rewrites_and_removes_paths_for_a_renamed_directory() {
        let old = Path::new("/repo/old");
        let new = Path::new("/repo/new");
        let mut problems = Problems::default();
        let mut diagnostic = lsp_diagnostic(1, "error", DiagnosticSeverity::Error);
        diagnostic.related.push(DiagnosticRelatedInformation {
            path: old.join("related.rs"),
            line: 2,
            message: "related".to_string(),
        });
        problems.replace_lsp("server", &old.join("a.rs"), vec![diagnostic]);
        problems.rewrite_paths(old, new);
        let visible = problems.visible(Some(&new.join("a.rs")));
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].path, new.join("a.rs"));
        assert_eq!(visible[0].related[0].path, new.join("related.rs"));
        problems.remove_paths(new);
        assert_eq!(problems.total(), 0);
    }

    #[test]
    fn task_sources_can_be_marked_stale_and_refreshed() {
        let mut problems = Problems::default();
        problems.push_task("check", task_finding(1));
        problems.mark_tasks_stale();
        assert!(problems.is_stale("task:check"));
        problems.push_task("check", task_finding(2));
        assert!(!problems.is_stale("task:check"));
        problems.mark_tasks_stale();
        problems.clear_source("task:check");
        assert!(!problems.is_stale("task:check"));
    }

    #[test]
    fn visible_problems_sort_by_severity_file_and_line() {
        let mut problems = Problems::default();
        problems.replace_lsp(
            "server",
            Path::new("b.rs"),
            vec![
                lsp_diagnostic(1, "warning", DiagnosticSeverity::Warning),
                lsp_diagnostic(2, "error", DiagnosticSeverity::Error),
            ],
        );
        problems.replace_lsp(
            "server",
            Path::new("a.rs"),
            vec![lsp_diagnostic(5, "error a", DiagnosticSeverity::Error)],
        );
        let visible = problems.visible(None);
        assert_eq!(visible[0].path, PathBuf::from("a.rs"));
        assert_eq!(visible[1].message, "error");
        assert_eq!(visible[2].message, "warning");
    }

    #[test]
    fn filters_apply_to_navigation() {
        let path = Path::new("src/main.rs");
        let mut problems = Problems::default();
        problems.replace_lsp(
            "server",
            path,
            vec![
                lsp_diagnostic(0, "error", DiagnosticSeverity::Error),
                lsp_diagnostic(1, "warning", DiagnosticSeverity::Warning),
            ],
        );
        problems.toggle_current_file();
        problems.cycle_severity();
        let (_, _, problem) = problems.advance(Some(path), true).unwrap();
        assert_eq!(problem.severity, ProblemSeverity::Error);
        problems.cycle_severity();
        assert_eq!(
            problems.advance(Some(path), true).unwrap().2.message,
            "warning"
        );
        problems.cycle_severity();
        assert!(problems.advance(Some(path), true).is_none());
    }

    #[test]
    fn navigation_wraps_and_follows_the_current_entry() {
        let path = Path::new("src/main.rs");
        let mut problems = Problems::default();
        problems.replace_lsp(
            "server",
            path,
            vec![
                lsp_diagnostic(0, "first", DiagnosticSeverity::Error),
                lsp_diagnostic(2, "second", DiagnosticSeverity::Error),
            ],
        );
        assert_eq!(problems.advance(Some(path), true).unwrap().0, 0);
        assert_eq!(problems.advance(Some(path), true).unwrap().0, 1);
        assert_eq!(problems.advance(Some(path), true).unwrap().0, 0);
        assert_eq!(problems.advance(Some(path), false).unwrap().0, 1);
    }

    #[test]
    fn edits_shift_lsp_ranges_until_the_next_publish() {
        let path = Path::new("src/main.rs");
        let mut problems = Problems::default();
        problems.replace_lsp(
            "server",
            path,
            vec![lsp_diagnostic(2, "stale", DiagnosticSeverity::Error)],
        );
        problems.observe_editor(Some(path), 1, Rope::from_str("one\ntwo\nthree\n"));
        problems.observe_editor(Some(path), 2, Rope::from_str("zero\none\ntwo\nthree\n"));
        let visible = problems.visible(None);
        assert_eq!((visible[0].line, visible[0].column), (4, 1));
    }

    #[test]
    fn deletions_collapse_ranges_until_the_next_publish() {
        let path = Path::new("src/main.rs");
        let mut problems = Problems::default();
        problems.replace_lsp(
            "server",
            path,
            vec![lsp_diagnostic(2, "stale", DiagnosticSeverity::Error)],
        );
        problems.observe_editor(Some(path), 1, Rope::from_str("one\ntwo\nthree\n"));
        problems.observe_editor(Some(path), 2, Rope::from_str("one\nthree\n"));
        let visible = problems.visible(None);
        assert_eq!(visible[0].line, 2);
    }

    #[test]
    fn task_problems_do_not_shift() {
        let path = Path::new("src/main.rs");
        let mut problems = Problems::default();
        problems.push_task("check", task_finding(3));
        problems.observe_editor(Some(path), 1, Rope::from_str("one\ntwo\nthree\n"));
        problems.observe_editor(Some(path), 2, Rope::from_str("zero\none\ntwo\nthree\n"));
        assert_eq!(problems.visible(None)[0].line, 3);
    }

    #[test]
    fn source_count_counts_task_and_lsp_sources() {
        let path = Path::new("src/main.rs");
        let mut problems = Problems::default();
        problems.push_task("check", task_finding(1));
        problems.replace_lsp(
            "rust-analyzer",
            path,
            vec![lsp_diagnostic(0, "lsp", DiagnosticSeverity::Error)],
        );
        assert_eq!(problems.source_count(), 2);
    }
}
