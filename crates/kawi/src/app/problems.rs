use kawi_buffer::{diff_ropes, RopeChange};
use kawi_lsp::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, DiagnosticTag};
use kawi_task as task;
use ropey::Rope;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::diagnostic_index::{DiagnosticCounts, DiagnosticIndex, DiagnosticNavigation};
use super::file_ops::{path_under, rewrite_path};

const MAX_PROBLEMS: usize = 10_000;
const TASK_PREFIX: &str = "task:";
const LSP_PREFIX: &str = "lsp:";
const PLUGIN_PREFIX: &str = "plugin:";

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

impl From<ProblemSeverity> for DiagnosticSeverity {
    fn from(severity: ProblemSeverity) -> Self {
        match severity {
            ProblemSeverity::Error => Self::Error,
            ProblemSeverity::Warning => Self::Warning,
            ProblemSeverity::Information => Self::Information,
            ProblemSeverity::Hint => Self::Hint,
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
    pub(super) code_description: Option<String>,
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
            code_description: None,
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
            message: sanitize_message(&diagnostic.message),
            code: diagnostic.code,
            code_description: diagnostic.code_description,
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
struct IndexCache {
    dirty: bool,
    index: DiagnosticIndex,
}

#[derive(Default)]
pub(super) struct Problems {
    groups: BTreeMap<GroupKey, Vec<Problem>>,
    filter: ProblemFilter,
    current: Option<ProblemKey>,
    editor: Option<EditorText>,
    stale_sources: BTreeSet<String>,
    source_ranks: BTreeMap<String, usize>,
    next_source_rank: usize,
    index_cache: RefCell<IndexCache>,
    revision: u64,
}

impl Problems {
    pub(super) fn clear_source(&mut self, source: &str) {
        self.stale_sources.remove(source);
        self.groups.retain(|key, _| key.source != source);
        self.invalidate_index();
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
        self.invalidate_index();
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
        self.invalidate_index();
    }

    pub(super) fn mark_tasks_stale(&mut self) {
        for key in self.groups.keys() {
            if key.source.starts_with(TASK_PREFIX) {
                self.stale_sources.insert(key.source.clone());
            }
        }
        self.invalidate_index();
    }

    pub(super) fn is_stale(&self, source: &str) -> bool {
        self.stale_sources.contains(source)
    }

    pub(super) fn clear_lsp_source(&mut self, server: &str) {
        self.clear_source(&lsp_source(server));
    }

    pub(super) fn clear_plugin(&mut self, plugin: &str) {
        self.clear_source(&plugin_source(plugin));
    }

    pub(super) fn clear_plugin_paths(&mut self, path: &Path) {
        self.groups
            .retain(|key, _| !key.source.starts_with(PLUGIN_PREFIX) || key.path != path);
    }

    pub(super) fn replace_plugin(&mut self, plugin: &str, path: &Path, problems: Vec<Problem>) {
        let key = GroupKey {
            source: plugin_source(plugin),
            path: path.to_path_buf(),
        };
        self.ensure_source_rank(&key.source);
        self.replace_group(key, problems);
    }

    pub(super) fn replace_lsp(&mut self, server: &str, path: &Path, diagnostics: Vec<Diagnostic>) {
        let key = GroupKey {
            source: lsp_source(server),
            path: path.to_path_buf(),
        };
        self.ensure_source_rank(&key.source);
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
        self.ensure_source_rank(&source);
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

    pub(super) fn problems_for_path(&self, path: &Path) -> Vec<&Problem> {
        self.groups
            .iter()
            .filter(|(key, _)| key.path == path)
            .flat_map(|(_, problems)| problems.iter())
            .collect()
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
        self.invalidate_index();
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
        self.invalidate_index();
        true
    }

    fn replace_group(&mut self, key: GroupKey, problems: Vec<Problem>) {
        let others = self
            .total()
            .saturating_sub(self.groups.get(&key).map_or(0, Vec::len));
        let capacity = MAX_PROBLEMS.saturating_sub(others);
        if problems.is_empty() || capacity == 0 {
            self.groups.remove(&key);
            self.invalidate_index();
            return;
        }
        let mut problems = problems;
        problems.truncate(capacity);
        self.groups.insert(key, problems);
        self.invalidate_index();
    }

    pub(super) fn diagnostics_at(&self, path: &Path, line: usize) -> Vec<Problem> {
        self.index().at(path, line).to_vec()
    }

    pub(super) fn primary_diagnostic(
        &self,
        path: &Path,
        line: usize,
        column: Option<usize>,
    ) -> Option<Problem> {
        self.index().primary(path, line, column).cloned()
    }

    pub(super) fn diagnostic_counts(&self, path: &Path) -> DiagnosticCounts {
        self.index().counts(path)
    }

    pub(super) fn workspace_diagnostic_counts(&self) -> DiagnosticCounts {
        self.index().workspace_counts()
    }

    pub(super) fn path_counts(&self) -> BTreeMap<PathBuf, DiagnosticCounts> {
        self.index().path_counts()
    }

    pub(super) fn revision(&self) -> u64 {
        self.revision
    }

    pub(super) fn navigate_lsp(
        &self,
        path: &Path,
        line: usize,
        column: usize,
        forward: bool,
    ) -> Option<DiagnosticNavigation> {
        self.index().navigate_lsp(path, line, column, forward)
    }

    fn ensure_source_rank(&mut self, source: &str) {
        if self.source_ranks.contains_key(source) {
            return;
        }
        self.source_ranks
            .insert(source.to_string(), self.next_source_rank);
        self.next_source_rank = self.next_source_rank.saturating_add(1);
    }

    fn invalidate_index(&mut self) {
        self.index_cache.get_mut().dirty = true;
        self.revision = self.revision.saturating_add(1);
    }

    fn index(&self) -> std::cell::RefMut<'_, DiagnosticIndex> {
        let mut cache = self.index_cache.borrow_mut();
        if cache.dirty {
            cache.index = DiagnosticIndex::build(
                self.groups.iter().flat_map(|(key, problems)| {
                    problems
                        .iter()
                        .map(move |problem| (key.path.as_path(), key.source.as_str(), problem))
                }),
                &self.source_ranks,
                &self.stale_sources,
            );
            cache.dirty = false;
        }
        std::cell::RefMut::map(cache, |cache: &mut IndexCache| &mut cache.index)
    }
}

pub(super) fn task_source(task_name: &str) -> String {
    format!("{TASK_PREFIX}{task_name}")
}

fn lsp_source(server: &str) -> String {
    format!("{LSP_PREFIX}{server}")
}

pub(super) fn plugin_source(plugin: &str) -> String {
    format!("{PLUGIN_PREFIX}{plugin}")
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

fn sanitize_message(message: &str) -> String {
    message
        .chars()
        .map(|character| {
            if character.is_control() && !matches!(character, '\n' | '\t') {
                ' '
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
