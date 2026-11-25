use std::time::{Duration, Instant};

use crate::error::PluginError;

const BASE_DELAY: Duration = Duration::from_millis(250);
const MAX_DELAY: Duration = Duration::from_secs(30);
const MAX_FAILURES: u32 = 5;

pub(crate) struct Backoff {
    failures: u32,
    delay: Duration,
    next_attempt: Instant,
}

impl Backoff {
    pub(crate) fn new() -> Self {
        Self {
            failures: 0,
            delay: BASE_DELAY,
            next_attempt: Instant::now(),
        }
    }

    pub(crate) fn check(&self) -> Result<(), PluginError> {
        if self.failures >= MAX_FAILURES {
            return Err(PluginError::Disabled);
        }
        let now = Instant::now();
        if now < self.next_attempt {
            return Err(PluginError::Backoff {
                remaining_ms: self.next_attempt.duration_since(now).as_millis() as u64,
            });
        }
        Ok(())
    }

    pub(crate) fn record_failure(&mut self) {
        self.failures = self.failures.saturating_add(1);
        self.next_attempt = Instant::now() + self.delay;
        self.delay = (self.delay * 2).min(MAX_DELAY);
    }

    pub(crate) fn record_success(&mut self) {
        self.failures = 0;
        self.delay = BASE_DELAY;
        self.next_attempt = Instant::now();
    }

    pub(crate) fn reset(&mut self) {
        self.record_success();
    }

    pub(crate) fn is_disabled(&self) -> bool {
        self.failures >= MAX_FAILURES
    }

    pub(crate) fn remaining(&self) -> Duration {
        self.next_attempt.saturating_duration_since(Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_back_off_and_disable() {
        let mut backoff = Backoff::new();
        backoff.record_failure();
        assert!(matches!(backoff.check(), Err(PluginError::Backoff { .. })));
        for _ in 0..4 {
            backoff.record_failure();
        }
        assert_eq!(backoff.check(), Err(PluginError::Disabled));
        assert!(backoff.is_disabled());
    }

    #[test]
    fn success_resets_the_backoff() {
        let mut backoff = Backoff::new();
        backoff.record_failure();
        backoff.record_success();
        assert_eq!(backoff.check(), Ok(()));
        assert_eq!(backoff.failures, 0);
    }
}
