use std::time::{Duration, Instant};

struct Scheduled<T, S> {
    deadline: Instant,
    site: S,
    value: T,
}

pub(crate) struct Delayed<T, S> {
    scheduled: Option<Scheduled<T, S>>,
}

impl<T, S> Delayed<T, S> {
    pub(crate) fn new() -> Self {
        Self { scheduled: None }
    }

    pub(super) fn schedule(&mut self, value: T, delay: Duration, site: S) {
        self.scheduled = Some(Scheduled {
            deadline: Instant::now() + delay,
            site,
            value,
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
}

impl<T, S: PartialEq> Delayed<T, S> {
    pub(super) fn take_due(&mut self, now: Instant, site: S) -> Option<T> {
        let scheduled = self.scheduled.as_ref()?;
        if scheduled.site != site {
            self.cancel();
            return None;
        }
        if scheduled.deadline > now {
            return None;
        }
        self.scheduled.take().map(|scheduled| scheduled.value)
    }
}

impl<T, S> Default for Delayed<T, S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn later() -> Instant {
        Instant::now() + Duration::from_secs(1)
    }

    #[test]
    fn due_value_fires_at_the_scheduled_site() {
        let mut delayed = Delayed::new();
        delayed.schedule("value", Duration::ZERO, 1);
        assert_eq!(delayed.take_due(later(), 1), Some("value"));
    }

    #[test]
    fn site_change_discards_the_value() {
        let mut delayed = Delayed::new();
        delayed.schedule("value", Duration::ZERO, 1);
        assert_eq!(delayed.take_due(later(), 2), None);
        assert!(delayed.wait_timeout().is_none());
    }

    #[test]
    fn value_waits_until_its_deadline() {
        let mut delayed = Delayed::new();
        delayed.schedule("value", Duration::from_secs(1), 1);
        assert!(!delayed.is_due(Instant::now()));
        assert!(delayed.wait_timeout().is_some());
    }

    #[test]
    fn scheduling_replaces_the_previous_value() {
        let mut delayed = Delayed::new();
        delayed.schedule("old", Duration::from_secs(1), 1);
        delayed.schedule("new", Duration::ZERO, 2);
        assert_eq!(delayed.take_due(later(), 2), Some("new"));
    }
}
