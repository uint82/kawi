use std::time::{Duration, Instant};

use kawi_completion::Trigger;

pub(crate) struct AutoTrigger {
    deadline: Option<Instant>,
    trigger: Option<Trigger>,
}

impl AutoTrigger {
    pub(crate) fn new() -> Self {
        Self {
            deadline: None,
            trigger: None,
        }
    }

    pub(super) fn schedule(&mut self, trigger: Trigger, delay: Duration) {
        self.trigger = Some(trigger);
        self.deadline = Some(Instant::now() + delay);
    }

    pub(crate) fn cancel(&mut self) {
        self.deadline = None;
        self.trigger = None;
    }

    pub(super) fn wait_timeout(&self) -> Option<Duration> {
        self.deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }

    pub(super) fn take_due(&mut self, now: Instant) -> Option<Trigger> {
        if self.deadline.is_some_and(|deadline| deadline <= now) {
            self.deadline = None;
            return self.trigger.take();
        }
        None
    }
}
