use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

pub(crate) struct WatchOptions {
    pub(crate) debounce: Duration,
    pub(crate) tick: Duration,
    pub(crate) clock: Arc<dyn Clock>,
}

impl WatchOptions {
    pub(crate) fn new(debounce: Duration, tick: Duration) -> Self {
        Self {
            debounce,
            tick,
            clock: Arc::new(SystemClock),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_clock(debounce: Duration, tick: Duration, clock: Arc<dyn Clock>) -> Self {
        Self {
            debounce,
            tick,
            clock,
        }
    }
}
