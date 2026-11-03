use super::*;

impl DebugUi {
    pub(super) fn apply_continued(&mut self) {
        self.status = DebugStatus::Running;
        self.clear_frame_state();
        self.pending_report = None;
        for watch in &mut self.watches {
            watch.clear();
        }
    }

    pub(super) fn apply_terminated(&mut self) {
        self.status = DebugStatus::Terminated;
        self.thread_id = None;
        self.clear_frame_state();
        self.pending_report = None;
        self.verified.clear();
        self.session.terminal_requests.clear();
        for watch in &mut self.watches {
            watch.clear();
        }
        self.message = Some("debug: terminated".to_string());
    }

    pub(super) fn apply_breakpoint(
        &mut self,
        source: Option<PathBuf>,
        line: Option<u32>,
        verified: bool,
    ) {
        let (Some(source), Some(line)) = (source, line) else {
            return;
        };
        let source = std::path::absolute(&source).unwrap_or(source);
        let line = line.saturating_sub(1);
        if verified {
            self.verified.insert((source, line));
        } else {
            self.verified.remove(&(source, line));
        }
    }
}
