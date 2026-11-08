use std::time::{Duration, Instant};

use kawi_completion::{Trigger, TriggerKind};

use super::prefix::completion_prefix;
use crate::app::App;

const AUTO_COMPLETION_DELAY: Duration = Duration::from_millis(120);

fn auto_completion_delay(trigger: &Trigger) -> Duration {
    match trigger.kind {
        TriggerKind::TriggerCharacter => Duration::ZERO,
        _ => AUTO_COMPLETION_DELAY,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct CompletionSite {
    pub(super) buffer: usize,
    pub(super) cursor: usize,
}

struct ScheduledTrigger {
    deadline: Instant,
    trigger: Trigger,
    site: CompletionSite,
}

pub(crate) struct AutoTrigger {
    scheduled: Option<ScheduledTrigger>,
}

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

impl AutoTrigger {
    pub(crate) fn new() -> Self {
        Self { scheduled: None }
    }

    pub(super) fn schedule(&mut self, trigger: Trigger, delay: Duration, site: CompletionSite) {
        self.scheduled = Some(ScheduledTrigger {
            deadline: Instant::now() + delay,
            trigger,
            site,
        });
    }

    pub(crate) fn cancel(&mut self) {
        self.scheduled = None;
    }

    pub(super) fn wait_timeout(&self) -> Option<Duration> {
        self.scheduled
            .as_ref()
            .map(|scheduled| scheduled.deadline.saturating_duration_since(Instant::now()))
    }

    pub(super) fn is_due(&self, now: Instant) -> bool {
        self.scheduled
            .as_ref()
            .is_some_and(|scheduled| scheduled.deadline <= now)
    }

    pub(super) fn take_due(&mut self, now: Instant, site: CompletionSite) -> Option<Trigger> {
        let scheduled = self.scheduled.as_ref()?;
        if scheduled.site != site {
            self.cancel();
            return None;
        }
        if scheduled.deadline > now {
            return None;
        }
        self.scheduled.take().map(|scheduled| scheduled.trigger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn later() -> Instant {
        Instant::now() + Duration::from_secs(1)
    }

    fn site(buffer: usize, cursor: usize) -> CompletionSite {
        CompletionSite { buffer, cursor }
    }

    #[test]
    fn due_trigger_fires_at_the_scheduled_site() {
        let mut auto = AutoTrigger::new();
        auto.schedule(Trigger::typing(), Duration::ZERO, site(1, 4));
        assert_eq!(auto.take_due(later(), site(1, 4)), Some(Trigger::typing()));
    }

    #[test]
    fn cursor_movement_discards_the_trigger() {
        let mut auto = AutoTrigger::new();
        auto.schedule(Trigger::typing(), Duration::ZERO, site(1, 4));
        assert_eq!(auto.take_due(later(), site(1, 3)), None);
        assert!(auto.wait_timeout().is_none());
    }

    #[test]
    fn buffer_switch_discards_the_trigger() {
        let mut auto = AutoTrigger::new();
        auto.schedule(Trigger::typing(), Duration::ZERO, site(1, 4));
        assert_eq!(auto.take_due(later(), site(2, 4)), None);
        assert!(auto.wait_timeout().is_none());
    }

    #[test]
    fn trigger_waits_until_its_deadline() {
        let mut auto = AutoTrigger::new();
        auto.schedule(Trigger::typing(), Duration::from_secs(1), site(1, 4));
        assert!(!auto.is_due(Instant::now()));
        assert!(auto.wait_timeout().is_some());
    }

    #[test]
    fn scheduling_replaces_the_previous_trigger() {
        let mut auto = AutoTrigger::new();
        auto.schedule(Trigger::typing(), Duration::from_secs(1), site(1, 1));
        auto.schedule(Trigger::character("."), Duration::ZERO, site(1, 2));
        assert_eq!(
            auto.take_due(later(), site(1, 2)),
            Some(Trigger::character("."))
        );
    }
}
