use super::lsp::DiagnosticUpdate;
use super::picker::PickerSession;
use super::problems::{code_label, source_label, Problem};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_picker::PickerRow;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const PANE_REFRESH_DELAY: Duration = Duration::from_millis(150);

impl super::App {
    pub(super) fn apply_diagnostic_updates(&mut self) -> bool {
        let updates = self.lsp.take_diagnostic_updates();
        let activity = self.lsp.take_diagnostics_activity();
        if updates.is_empty() && !activity {
            return false;
        }
        for update in updates {
            match update {
                DiagnosticUpdate::Replace(event) => {
                    self.problems.replace_lsp(event);
                }
                DiagnosticUpdate::Clear(server) => self.problems.clear_lsp_source(&server),
            }
        }
        self.mark_problems_changed();
        true
    }

    pub(super) fn mark_problems_changed(&mut self) {
        if !self.picker.as_ref().is_some_and(PickerSession::is_quickfix) {
            return;
        }
        self.problem_pane_dirty = true;
        if self.problem_pane_deadline.is_none() {
            self.problem_pane_deadline = Some(Instant::now() + PANE_REFRESH_DELAY);
        }
    }

    pub(super) fn problem_pane_tick(&mut self) -> bool {
        if !self.problem_pane_dirty {
            return false;
        }
        if self
            .problem_pane_deadline
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return false;
        }
        self.problem_pane_dirty = false;
        self.problem_pane_deadline = None;
        let Some(mut session) = self.picker.take() else {
            return false;
        };
        if !session.is_quickfix() {
            self.picker = Some(session);
            return false;
        }
        self.refresh_quickfix_picker(&mut session);
        self.picker = Some(session);
        true
    }

    pub(super) fn problem_pane_wait_timeout(&self) -> Option<Duration> {
        if !self.problem_pane_dirty {
            return None;
        }
        self.problem_pane_deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }

    pub(super) fn handle_quickfix_command(
        &mut self,
        command: kawi_editor::QuickfixCommand,
    ) -> anyhow::Result<bool> {
        match command {
            kawi_editor::QuickfixCommand::Open => self.open_quickfix_picker(),
            kawi_editor::QuickfixCommand::Next => self.handle_quickfix_navigation(true),
            kawi_editor::QuickfixCommand::Previous => self.handle_quickfix_navigation(false),
            kawi_editor::QuickfixCommand::Close => {
                self.picker = None;
                self.problem_pane_dirty = false;
                self.problem_pane_deadline = None;
                Ok(false)
            }
        }
    }

    pub(super) fn handle_quickfix_filter_key(
        &mut self,
        session: &mut PickerSession,
        key: KeyEvent,
    ) -> bool {
        if !session.is_quickfix() || key.modifiers != KeyModifiers::CONTROL {
            return false;
        }
        match key.code {
            KeyCode::Char('e') => self.problems.cycle_severity(),
            KeyCode::Char('f') => self.problems.toggle_current_file(),
            _ => return false,
        }
        self.refresh_quickfix_picker(session);
        true
    }

    fn open_quickfix_picker(&mut self) -> anyhow::Result<bool> {
        self.lsp.request_workspace_diagnostics();
        let root = self.project_root();
        let mut session = PickerSession::quickfix(root.clone(), Vec::new());
        self.refresh_quickfix_picker(&mut session);
        self.pager = None;
        self.picker = Some(session);
        self.problem_pane_dirty = false;
        self.problem_pane_deadline = None;
        Ok(false)
    }

    fn handle_quickfix_navigation(&mut self, forward: bool) -> anyhow::Result<bool> {
        let path = self.problem_path();
        let Some((index, total, problem)) = self.problems.advance(path.as_deref(), forward) else {
            self.show_message("Problems list is empty".to_string());
            return Ok(false);
        };
        if let Err(message) = self.open_file_location(
            &problem.path,
            Some(problem.line - 1),
            Some(problem.column - 1),
        ) {
            self.show_message(message);
            return Ok(false);
        }
        self.show_message(format!(
            "Problem {}/{} [{}]: {}",
            index.saturating_add(1),
            total,
            source_label(&problem.source),
            super::compact_error(&problem.message)
        ));
        Ok(false)
    }

    fn refresh_quickfix_picker(&mut self, session: &mut PickerSession) {
        let root = session.root().to_path_buf();
        let path = self.problem_path();
        let visible = self.problems.visible(path.as_deref());
        let rows = visible
            .iter()
            .map(|problem| problem_row(&root, problem, self.problems.is_stale(&problem.source)))
            .collect();
        let status = self.problem_status(visible.len());
        session.set_locations(rows);
        session.set_status(Some(status));
    }

    fn problem_status(&self, shown: usize) -> String {
        let total = self.problems.total();
        let filter = self.problems.filter();
        let files = if filter.current_file_only() {
            "current file"
        } else {
            "all files"
        };
        let lsp = if self.lsp.diagnostics_loading() {
            format!("{} scanning", self.lsp.diagnostic_mode_label())
        } else {
            self.lsp.diagnostic_mode_label().to_string()
        };
        format!(
            "{shown} shown, {total} total, {} sources  severity {}  {files}  LSP {lsp}  Ctrl-E severity  Ctrl-F file",
            self.problems.source_count(),
            filter.severity_label()
        )
    }

    pub(super) fn problem_path(&self) -> Option<PathBuf> {
        self.buffer_problem_path(self.editor.buffer_number())
    }

    pub(super) fn buffer_problem_path(&self, buffer: usize) -> Option<PathBuf> {
        let path = self.editor.buffer_path(buffer)?;
        let cwd = std::env::current_dir().unwrap_or_default();
        Some(kawi_editor::paths::absolute(path, &cwd))
    }

    #[cfg(test)]
    pub(super) fn diagnostic_status_text(&self, path: Option<&Path>) -> Option<String> {
        let path = path?;
        self.diagnostic_options.counts_text(
            self.problems.diagnostic_counts(path),
            self.problems.workspace_diagnostic_counts(),
        )
    }
}

fn problem_row(root: &Path, problem: &Problem, stale: bool) -> PickerRow {
    let path = problem.path.strip_prefix(root).unwrap_or(&problem.path);
    let code = code_label(problem)
        .map(|label| format!(" [{label}]"))
        .unwrap_or_default();
    let tags = problem
        .tags
        .iter()
        .map(|tag| format!(" [{}]", tag.label()))
        .collect::<String>();
    let related = match problem.related.len() {
        0 => String::new(),
        count => format!(" (+{count} related)"),
    };
    let stale = if stale { " [stale]" } else { "" };
    PickerRow {
        display: format!(
            "{}:{}:{} {} [{}]{code}{tags}{stale} {}{related}",
            path.display(),
            problem.line,
            problem.column,
            problem.severity.label(),
            source_label(&problem.source),
            crate::text::one_line(&problem.message)
        ),
        path: problem.path.to_string_lossy().to_string(),
        line: Some(problem.line),
        column: Some(problem.column),
        buffer: None,
        secondary: None,
        deprecated: false,
        id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::App;
    use crate::keymap::Flavor;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use kawi_buffer::Buffer;
    use kawi_lsp::{
        Diagnostic, DiagnosticPosition, DiagnosticRange, DiagnosticRelatedInformation,
        DiagnosticSeverity, DiagnosticTag,
    };
    use kawi_task::{Finding, Severity};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_quickfix_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn finding(path: &Path, line: usize, message: &str) -> Finding {
        Finding {
            path: path.to_path_buf(),
            line,
            column: 1,
            severity: Severity::Error,
            message: message.to_string(),
        }
    }

    fn lsp_diagnostic(path: &Path) -> Diagnostic {
        Diagnostic {
            range: DiagnosticRange {
                start: DiagnosticPosition {
                    line: 0,
                    character: 0,
                },
                end: DiagnosticPosition {
                    line: 0,
                    character: 1,
                },
            },
            severity: DiagnosticSeverity::Error,
            message: "type mismatch".to_string(),
            source: Some("rustc".to_string()),
            code: Some("E0308".to_string()),
            code_description: None,
            tags: vec![DiagnosticTag::Unnecessary],
            related: vec![DiagnosticRelatedInformation {
                path: path.to_path_buf(),
                line: 2,
                message: "expected here".to_string(),
            }],
        }
    }

    fn app_with_problem(name: &str, line: usize) -> (App, PathBuf) {
        let dir = temp_dir(name);
        let file = dir.join("a.txt");
        fs::write(&file, "zero\none\ntwo\n").unwrap();
        let mut app = App::new(Buffer::from_string("text"), Flavor::Vim);
        app.problems
            .push_task("check", finding(&file, line, "problem"));
        (app, dir)
    }

    #[test]
    fn navigation_opens_files_and_wraps() {
        let (mut app, dir) = app_with_problem("navigation", 2);
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Next)
            .unwrap();
        assert_eq!(app.cursor_position(), (1, 0));
        assert!(app.last_message().unwrap().starts_with("Problem 1/1"));
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Next)
            .unwrap();
        assert_eq!(app.cursor_position(), (1, 0));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn open_lists_problems_with_source_labels() {
        let (mut app, dir) = app_with_problem("open", 1);
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Open)
            .unwrap();
        let picker = app.picker.as_ref().unwrap();
        assert_eq!(picker.picker().title(), "Problems");
        assert_eq!(picker.picker().rows().len(), 1);
        assert!(picker.picker().rows()[0].display.contains("[task:check]"));
        assert!(picker.picker().status().unwrap().contains("LSP off"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn empty_problems_still_show_the_pane_and_mode() {
        let mut app = App::new(Buffer::from_string("text"), Flavor::Vim);
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Open)
            .unwrap();
        let picker = app.picker.as_ref().unwrap();
        assert_eq!(picker.picker().rows().len(), 0);
        assert!(picker.picker().status().unwrap().contains("0 shown"));
        assert_eq!(
            app.last_message(),
            None,
            "opening the pane should not report an error"
        );
    }

    #[test]
    fn severity_filter_applies_to_pane_and_navigation() {
        let (mut app, dir) = app_with_problem("filters", 1);
        app.problems
            .push_task("check", finding(&dir.join("a.txt"), 3, "second"));
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Open)
            .unwrap();
        app.handle_picker_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL))
            .unwrap();
        let status = app
            .picker
            .as_ref()
            .unwrap()
            .picker()
            .status()
            .unwrap()
            .to_string();
        assert!(status.contains("severity errors"));
        assert_eq!(app.picker.as_ref().unwrap().picker().rows().len(), 2);
        app.problems.cycle_severity();
        app.problems.cycle_severity();
        app.problems.cycle_severity();
        assert!(app.problems.visible(None).is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn navigation_respects_current_file_filter() {
        let dir = temp_dir("current-file");
        let first = dir.join("a.txt");
        let second = dir.join("b.txt");
        fs::write(&first, "a\n").unwrap();
        fs::write(&second, "b\n").unwrap();
        let mut app = App::new(Buffer::from_string("text"), Flavor::Vim);
        app.problems.push_task("check", finding(&first, 1, "first"));
        app.problems
            .push_task("check", finding(&second, 1, "second"));
        app.problems.toggle_current_file();
        assert!(app
            .handle_quickfix_command(kawi_editor::QuickfixCommand::Next)
            .is_ok());
        assert_eq!(app.last_message(), Some("Problems list is empty"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lsp_rows_show_source_code_tags_and_related_data() {
        let dir = temp_dir("lsp-row");
        let file = dir.join("a.rs");
        fs::write(&file, "one\ntwo\nthree\n").unwrap();
        let mut app = App::new(Buffer::from_string("text"), Flavor::Vim);
        app.problems
            .replace_lsp_test("rust-analyzer", &file, vec![lsp_diagnostic(&file)]);
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Open)
            .unwrap();
        let row = &app.picker.as_ref().unwrap().picker().rows()[0].display;
        assert!(row.contains("[lsp:rust-analyzer]"), "{row}");
        assert!(row.contains("[rustc E0308]"), "{row}");
        assert!(row.contains("[unused]"), "{row}");
        assert!(row.contains("(+1 related)"), "{row}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pane_refresh_is_debounced_and_updates_the_open_list() {
        let (mut app, dir) = app_with_problem("debounce", 1);
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Open)
            .unwrap();
        assert_eq!(app.picker.as_ref().unwrap().picker().rows().len(), 1);
        app.problems
            .push_task("check", finding(&dir.join("a.txt"), 2, "third"));
        app.mark_problems_changed();
        assert!(app.problem_pane_wait_timeout().is_some());
        assert!(!app.problem_pane_tick());
        std::thread::sleep(Duration::from_millis(160));
        assert!(app.problem_pane_tick());
        assert_eq!(app.picker.as_ref().unwrap().picker().rows().len(), 2);
        assert!(app.problem_pane_wait_timeout().is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn empty_navigation_reports_without_opening_files() {
        let mut app = App::new(Buffer::from_string("text"), Flavor::Vim);
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Next)
            .unwrap();
        assert_eq!(app.last_message(), Some("Problems list is empty"));
    }
}
