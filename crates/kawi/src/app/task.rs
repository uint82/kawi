use super::{compact_error, App};
use kawi_editor::TaskCommand;
use kawi_task::{OutputStream, TaskEvent, TaskRunner, TaskSpec, TaskStatus, Tasks};
use kawi_tui::frame::{Rect, TaskLine};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_OUTPUT_LINES: usize = 10_000;
const PANEL_MIN_HEIGHT: usize = 4;
const PANEL_MAX_HEIGHT: usize = 12;
pub(super) const PROJECT_TASKS: &str = ".kawi/tasks.toml";

#[derive(Debug)]
pub(super) struct TaskOutputLine {
    pub(super) text: String,
    pub(super) stderr: bool,
}

#[derive(Default)]
pub(super) struct TaskSession {
    runner: Option<TaskRunner>,
    output: VecDeque<TaskOutputLine>,
    name: Option<String>,
    status: String,
    last_task: Option<String>,
    findings: usize,
    panel_visible: bool,
}

impl TaskSession {
    pub(super) fn begin(&mut self, spec: &TaskSpec) {
        self.output.clear();
        self.name = Some(spec.name.clone());
        self.last_task = Some(spec.name.clone());
        self.status = "running".to_string();
        self.findings = 0;
        self.panel_visible = true;
        self.push_output(OutputStream::Stdout, command_line(spec));
    }

    pub(super) fn attach(&mut self, runner: TaskRunner) {
        self.runner = Some(runner);
    }

    pub(super) fn runner(&self) -> Option<&TaskRunner> {
        self.runner.as_ref()
    }

    pub(super) fn is_running(&self) -> bool {
        self.runner.is_some()
    }

    pub(super) fn last_task(&self) -> Option<&str> {
        self.last_task.as_deref()
    }

    pub(super) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub(super) fn status(&self) -> &str {
        &self.status
    }

    pub(super) fn findings(&self) -> usize {
        self.findings
    }

    pub(super) fn push_output(&mut self, stream: OutputStream, text: String) {
        if self.output.len() == MAX_OUTPUT_LINES {
            self.output.pop_front();
        }
        self.output.push_back(TaskOutputLine {
            text,
            stderr: stream == OutputStream::Stderr,
        });
    }

    pub(super) fn count_finding(&mut self) {
        self.findings = self.findings.saturating_add(1);
    }

    pub(super) fn finish(&mut self, status: TaskStatus, description: &str) {
        self.runner = None;
        self.status = status_description(status, description);
    }

    pub(super) fn fail_start(&mut self, error: String) {
        self.runner = None;
        self.status = "failed to start".to_string();
        self.push_output(OutputStream::Stderr, error);
    }

    pub(super) fn cancel(&self) -> bool {
        let Some(runner) = self.runner.as_ref() else {
            return false;
        };
        runner.cancel();
        true
    }

    pub(super) fn toggle_panel(&mut self) -> bool {
        if self.name.is_none() {
            return false;
        }
        self.panel_visible = !self.panel_visible;
        true
    }

    pub(super) fn close_panel(&mut self) {
        self.panel_visible = false;
    }

    pub(super) fn panel_height(&self, screen_height: usize) -> usize {
        if !self.panel_visible || self.name.is_none() {
            return 0;
        }
        let available = screen_height.saturating_sub(2);
        if available < PANEL_MIN_HEIGHT {
            return 0;
        }
        (screen_height / 3)
            .clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT)
            .min(available)
    }

    pub(super) fn output_tail(&self, rows: usize) -> impl Iterator<Item = &TaskOutputLine> {
        let start = self.output.len().saturating_sub(rows);
        self.output.iter().skip(start)
    }
}

impl App {
    pub(super) fn set_tasks(&mut self, tasks: Tasks) {
        self.tasks = tasks;
    }

    pub(super) fn task_root(&self) -> PathBuf {
        let cwd = std::env::current_dir().unwrap_or_default();
        let cwd_root = kawi_picker::project_root(&cwd);
        if !self.editor.has_file() {
            return cwd_root;
        }
        let file_root = kawi_picker::project_root(Path::new(self.editor.filename()));
        if file_root.join(".git").exists() {
            file_root
        } else {
            cwd_root
        }
    }

    pub(super) fn task_panel_rect(&self) -> Option<Rect> {
        let height = self.task.panel_height(self.screen_height);
        if height == 0 {
            return None;
        }
        let terminal_height = self.terminal.panel_height(self.screen_height, height);
        let terminal_width = self.terminal.panel_width(self.screen_width);
        Some(Rect::new(
            0,
            self.screen_height
                .saturating_sub(1)
                .saturating_sub(terminal_height)
                .saturating_sub(height),
            self.screen_width.saturating_sub(terminal_width),
            height,
        ))
    }

    pub(super) fn task_panel_title(&self) -> Option<String> {
        self.task.name().map(|name| format!("Task: {name}"))
    }

    pub(super) fn task_panel_status(&self) -> Option<&str> {
        self.task.name().map(|_| self.task.status())
    }

    pub(super) fn task_panel_lines(&self, rows: usize) -> Vec<TaskLine<'_>> {
        self.task
            .output_tail(rows)
            .map(|line| TaskLine {
                text: &line.text,
                stderr: line.stderr,
            })
            .collect()
    }

    pub(super) fn handle_task_command(&mut self, command: TaskCommand) -> anyhow::Result<bool> {
        match command {
            TaskCommand::Run(name) => self.start_task(&name),
            TaskCommand::List => {
                self.list_tasks();
                Ok(false)
            }
            TaskCommand::Rerun => self.rerun_task(),
            TaskCommand::Cancel => {
                self.cancel_task();
                Ok(false)
            }
            TaskCommand::Output => {
                self.toggle_task_output();
                Ok(false)
            }
            TaskCommand::Close => {
                self.close_task_output();
                Ok(false)
            }
        }
    }

    pub(super) fn sync_task(&mut self) -> bool {
        let mut events = Vec::new();
        if let Some(runner) = self.task.runner() {
            while let Some(event) = runner.try_recv() {
                events.push(event);
            }
        }
        let mut changed = false;
        for event in events {
            changed |= self.apply_task_event(event);
        }
        changed
    }

    fn start_task(&mut self, name: &str) -> anyhow::Result<bool> {
        if self.task.is_running() {
            self.show_message("E: A task is already running".to_string());
            return Ok(false);
        }
        let Some(spec) = self.tasks.get(name).cloned() else {
            self.show_message(format!("E: Unknown task: {name}"));
            return Ok(false);
        };
        let root = self.task_root();
        self.problems
            .clear_source(&super::problems::task_source(name));
        self.mark_problems_changed();
        self.task.begin(&spec);
        match TaskRunner::start(&spec, &root, self.events.wake()) {
            Ok(runner) => {
                self.task.attach(runner);
                self.sync_task_panel_size();
            }
            Err(error) => {
                let message = compact_error(&error.to_string());
                self.task.fail_start(message.clone());
                self.sync_task_panel_size();
                self.show_message(format!("Task {name} failed to start: {message}"));
            }
        }
        Ok(false)
    }

    fn rerun_task(&mut self) -> anyhow::Result<bool> {
        let Some(name) = self.task.last_task().map(str::to_string) else {
            self.show_message("E: No previous task".to_string());
            return Ok(false);
        };
        self.start_task(&name)
    }

    fn cancel_task(&mut self) {
        if self.task.cancel() {
            self.show_message("Task cancellation requested".to_string());
        } else {
            self.show_message("E: No task is running".to_string());
        }
    }

    fn toggle_task_output(&mut self) {
        if self.task.toggle_panel() {
            self.sync_task_panel_size();
        } else {
            self.show_message("E: No task output".to_string());
        }
    }

    fn close_task_output(&mut self) {
        self.task.close_panel();
        self.sync_task_panel_size();
    }

    fn list_tasks(&mut self) {
        if self.tasks.is_empty() {
            self.show_message("No tasks configured".to_string());
            return;
        }
        let lines = self
            .tasks
            .names()
            .filter_map(|name| self.tasks.get(name).map(|task| (name, task)))
            .map(|(name, task)| {
                let mut command = task.command.clone();
                for arg in &task.args {
                    command.push(' ');
                    command.push_str(arg);
                }
                format!("{name}: {command}")
            })
            .collect::<Vec<_>>();
        self.show_message(lines.join("\n"));
    }

    fn apply_task_event(&mut self, event: TaskEvent) -> bool {
        match event {
            TaskEvent::Output { stream, line } => {
                self.task.push_output(stream, line);
                true
            }
            TaskEvent::Finding(finding) => {
                let Some(name) = self.task.name().map(str::to_string) else {
                    return false;
                };
                self.problems.push_task(&name, finding);
                self.task.count_finding();
                self.mark_problems_changed();
                true
            }
            TaskEvent::Finished {
                status,
                description,
            } => {
                self.task.finish(status, &description);
                self.sync_task_panel_size();
                self.report_task_finished(&description);
                true
            }
        }
    }

    fn report_task_finished(&mut self, description: &str) {
        let name = self.task.name().unwrap_or("task").to_string();
        let findings = self.task.findings();
        let suffix = match findings {
            0 => String::new(),
            1 => " (1 finding)".to_string(),
            count => format!(" ({count} findings)"),
        };
        self.show_message(format!("Task {name}: {description}{suffix}"));
    }

    fn sync_task_panel_size(&mut self) {
        self.sync_terminal_panel_size();
    }
}

fn command_line(spec: &TaskSpec) -> String {
    let mut line = format!("$ {}", spec.command);
    for arg in &spec.args {
        line.push(' ');
        line.push_str(arg);
    }
    line
}

fn status_description(status: TaskStatus, description: &str) -> String {
    format!("{description} in {}", duration_label(status.duration))
}

fn duration_label(duration: Duration) -> String {
    if duration.as_secs() > 0 {
        format!("{:.1}s", duration.as_secs_f64())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;
    use std::fs;
    use std::path::PathBuf;
    use std::thread;
    use std::time::Instant;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_task_app_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn app_with_tasks(name: &str, text: &str) -> (App, PathBuf) {
        let dir = temp_dir(name);
        let path = dir.join("tasks.toml");
        fs::write(&path, text).unwrap();
        let loaded = kawi_task::load(&path);
        assert!(loaded.warnings.is_empty());
        let mut app = App::new(Buffer::from_string("text"), Flavor::Vim);
        app.set_tasks(loaded.tasks);
        (app, dir)
    }

    fn finish(app: &mut App) {
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while app.task.is_running() {
            assert!(Instant::now() < deadline, "task did not finish");
            app.sync_task();
            thread::sleep(std::time::Duration::from_millis(5));
        }
        app.sync_task();
    }

    #[test]
    #[cfg(unix)]
    fn runs_a_task_and_collects_findings() {
        let (mut app, dir) = app_with_tasks(
            "run",
            r#"
[task.check]
command = "sh"
args = ["-c", "echo src/a.go:2:3: bad; exit 1"]
parser = "go"
"#,
        );
        app.start_task("check").unwrap();
        assert!(app.task.is_running());
        assert!(app.task_panel_rect().is_some());
        finish(&mut app);
        assert_eq!(app.problems.total(), 1);
        assert_eq!(app.problems.visible(None)[0].line, 2);
        assert_eq!(app.task.findings(), 1);
        assert!(app.task.status().starts_with("exit 1"));
        let output = app.task_panel_lines(100);
        assert!(output
            .iter()
            .any(|line| line.text == "$ sh -c echo src/a.go:2:3: bad; exit 1"));
        assert!(output.iter().any(|line| line.text.contains("src/a.go:2:3")));
        assert!(app.last_message().unwrap().contains("(1 finding)"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    #[cfg(unix)]
    fn reruns_the_last_task() {
        let (mut app, dir) = app_with_tasks(
            "rerun",
            r#"
[task.echo]
command = "sh"
args = ["-c", "echo value"]
"#,
        );
        app.start_task("echo").unwrap();
        finish(&mut app);
        app.rerun_task().unwrap();
        assert!(app.task.is_running());
        finish(&mut app);
        assert_eq!(app.task.last_task(), Some("echo"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    #[cfg(unix)]
    fn cancels_a_running_task() {
        let (mut app, dir) = app_with_tasks(
            "cancel",
            r#"
[task.sleep]
command = "sh"
args = ["-c", "sleep 10"]
"#,
        );
        app.start_task("sleep").unwrap();
        thread::sleep(std::time::Duration::from_millis(20));
        app.cancel_task();
        finish(&mut app);
        assert!(app.task.status().starts_with("cancelled"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unknown_task_and_empty_state_report_errors() {
        let mut app = App::new(Buffer::from_string("text"), Flavor::Vim);
        app.start_task("missing").unwrap();
        assert_eq!(app.last_message(), Some("E: Unknown task: missing"));
        app.rerun_task().unwrap();
        assert_eq!(app.last_message(), Some("E: No previous task"));
        app.cancel_task();
        assert_eq!(app.last_message(), Some("E: No task is running"));
        app.toggle_task_output();
        assert_eq!(app.last_message(), Some("E: No task output"));
    }

    #[test]
    fn panel_height_reserves_space_and_closes() {
        let mut session = TaskSession::default();
        assert_eq!(session.panel_height(24), 0);
        let spec = TaskSpec {
            name: "check".to_string(),
            command: "true".to_string(),
            args: Vec::new(),
            cwd: None,
            env: Default::default(),
            parser: kawi_task::ParserSpec::None,
        };
        session.begin(&spec);
        assert_eq!(session.panel_height(24), 8);
        assert!(session.toggle_panel());
        assert_eq!(session.panel_height(24), 0);
        session.close_panel();
        assert_eq!(session.panel_height(24), 0);
    }
}
