use std::collections::VecDeque;

use crate::protocol::{ProcessJobEvent, MAX_JOB_EVENT_QUEUE_BYTES, MAX_JOB_EVENT_QUEUE_ITEMS};

pub(super) struct JobEventQueue {
    events: VecDeque<ProcessJobEvent>,
    bytes: usize,
}

impl JobEventQueue {
    pub(super) fn new() -> Self {
        Self {
            events: VecDeque::new(),
            bytes: 0,
        }
    }

    pub(super) fn push(&mut self, event: ProcessJobEvent) {
        let size = event.size();
        while !self.events.is_empty()
            && (self.events.len() >= MAX_JOB_EVENT_QUEUE_ITEMS
                || self.bytes + size > MAX_JOB_EVENT_QUEUE_BYTES)
        {
            if let Some(removed) = self.events.pop_front() {
                self.bytes = self.bytes.saturating_sub(removed.size());
            }
        }
        if size <= MAX_JOB_EVENT_QUEUE_BYTES {
            self.events.push_back(event);
            self.bytes += size;
        }
    }

    pub(super) fn drain(&mut self) -> Vec<ProcessJobEvent> {
        self.bytes = 0;
        self.events.drain(..).collect()
    }
}
