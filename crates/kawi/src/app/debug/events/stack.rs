use super::*;

impl DebugUi {
    pub(super) fn apply_stopped(
        &mut self,
        reason: String,
        thread_id: Option<i64>,
        all_threads_stopped: bool,
        description: Option<String>,
        text: Option<String>,
    ) {
        self.status = DebugStatus::Stopped;
        self.thread_id = thread_id;
        self.clear_frame_state();
        self.jump_on_stack = true;
        let detail = text.or(description).unwrap_or_else(|| {
            if all_threads_stopped {
                reason.clone()
            } else {
                format!("{reason} (one thread)")
            }
        });
        self.message = Some(format!("debug: paused: {detail}"));
        if let (Some(thread_id), Some(service)) = (thread_id, self.service.as_ref()) {
            service.stack_trace(thread_id);
        }
    }
}
