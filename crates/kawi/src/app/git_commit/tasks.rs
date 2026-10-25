use kawi_task::{TaskOptions, TaskSpec, TaskStatus};

use super::{ActiveTask, GitTask, TaskKind};
use crate::app::App;

impl App {
    pub(crate) fn start_task_spec(
        &mut self,
        spec: kawi_task::TaskSpec,
        root: std::path::PathBuf,
        options: TaskOptions,
    ) -> bool {
        if self.task.is_running() {
            self.show_message("E: A task is already running".to_string());
            return false;
        }
        self.problems
            .clear_source(&super::super::problems::task_source(&spec.name));
        self.mark_problems_changed();
        self.task.begin(&spec);
        match kawi_task::TaskRunner::start_with(&spec, &root, self.events.wake(), options) {
            Ok(runner) => {
                self.task.attach(runner);
                self.sync_task_panel_size();
                true
            }
            Err(error) => {
                let message = super::super::compact_error(&error.to_string());
                self.task.fail_start(message.clone());
                self.sync_task_panel_size();
                self.show_message(format!("Task {} failed to start: {message}", spec.name));
                false
            }
        }
    }
}

impl App {
    pub(super) fn start_git_task(&mut self, task: GitTask) -> bool {
        let spec = TaskSpec {
            name: task.name,
            command: "git".to_string(),
            args: task.args,
            cwd: None,
            env: task.env,
            parser: task.parser,
        };
        if !self.start_task_spec(
            spec,
            task.root,
            TaskOptions {
                detached: task.detached,
            },
        ) {
            return false;
        }
        self.git_commit.active = Some(ActiveTask { kind: task.kind });
        true
    }

    pub(crate) fn finish_git_task(&mut self, status: TaskStatus) {
        let Some(kind) = self.git_commit.take_active() else {
            return;
        };
        if self.git_commit.editor.is_some() {
            self.abort_git_editor();
        }
        match kind {
            TaskKind::Commit | TaskKind::Amend => {
                self.reload_open_files();
                let _ = self.handle_git_refresh();
                self.refresh_git_status();
            }
            TaskKind::Push | TaskKind::Fetch => self.refresh_git_status(),
        }
        if !status.success && !status.cancelled {
            self.show_message(failure_message(kind));
        }
    }

    fn reload_open_files(&mut self) {
        self.sync_watched_files();
        let paths = self
            .open_files()
            .into_iter()
            .map(|(_, path)| path)
            .collect::<Vec<_>>();
        self.handle_disk_paths(&paths);
    }
}

fn failure_message(kind: TaskKind) -> String {
    match kind {
        TaskKind::Commit => "Commit failed; see the task output".to_string(),
        TaskKind::Amend => "Amend failed; see the task output".to_string(),
        TaskKind::Push => "Push failed; run :term git push for interactive auth".to_string(),
        TaskKind::Fetch => "Fetch failed; see the task output".to_string(),
    }
}
