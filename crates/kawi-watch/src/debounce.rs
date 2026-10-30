use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub(crate) struct Debounce {
    window: Duration,
    pending: BTreeSet<PathBuf>,
    deadline: Option<Instant>,
}

impl Debounce {
    pub(crate) fn new(window: Duration) -> Self {
        Self {
            window,
            pending: BTreeSet::new(),
            deadline: None,
        }
    }

    pub(crate) fn push(&mut self, paths: Vec<PathBuf>, now: Instant) {
        if paths.is_empty() {
            return;
        }
        self.pending.extend(paths);
        self.deadline = Some(now + self.window);
    }

    pub(crate) fn take_ready(&mut self, now: Instant) -> Option<Vec<PathBuf>> {
        let deadline = self.deadline?;
        if now < deadline {
            return None;
        }
        self.deadline = None;
        Some(std::mem::take(&mut self.pending).into_iter().collect())
    }

    pub(crate) fn wait_timeout(&self, now: Instant, tick: Duration) -> Duration {
        match self.deadline {
            Some(deadline) => deadline.saturating_duration_since(now).min(tick),
            None => tick,
        }
    }
}
