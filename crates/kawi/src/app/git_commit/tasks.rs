use kawi_task::TaskOptions;

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
