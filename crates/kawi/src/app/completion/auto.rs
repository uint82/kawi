use std::time::{Duration, Instant};

use kawi_completion::{Trigger, TriggerKind};

use super::prefix::completion_prefix;
use crate::app::delayed::Delayed;
use crate::app::App;

const AUTO_COMPLETION_DELAY: Duration = Duration::from_millis(120);

fn auto_completion_delay(trigger: &Trigger) -> Duration {
    match trigger.kind {
        TriggerKind::TriggerCharacter => Duration::ZERO,
        _ => AUTO_COMPLETION_DELAY,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::app) struct CompletionSite {
    pub(super) buffer: usize,
    pub(super) cursor: usize,
}

pub(crate) type AutoTrigger = Delayed<Trigger, CompletionSite>;

impl App {
    pub(in crate::app) fn completion_wait_timeout(&self) -> Option<Duration> {
        self.completion_auto.wait_timeout()
    }

    pub(in crate::app) fn completion_tick(&mut self) -> bool {
        let now = Instant::now();
        if !self.completion_auto.is_due(now) {
            return false;
        }
        let site = CompletionSite {
            buffer: self.editor.buffer_number(),
            cursor: completion_prefix(&self.editor).cursor,
        };
        let Some(trigger) = self.completion_auto.take_due(now, site) else {
            return false;
        };
        if !self.completion_request_allowed() {
            return false;
        }
        self.start_completion(trigger, false);
        true
    }

    pub(super) fn schedule_auto_completion(&mut self, trigger: Option<Trigger>, cursor: usize) {
        if !self.completion_options.auto || !self.completion_request_allowed() {
            return;
        }
        if let Some(trigger) = trigger {
            let delay = auto_completion_delay(&trigger);
            let site = CompletionSite {
                buffer: self.editor.buffer_number(),
                cursor,
            };
            self.completion_auto.schedule(trigger, delay, site);
            self.lsp.cancel_completion();
        } else if self.completion.is_none() {
            self.completion_auto.cancel();
        }
    }
}
