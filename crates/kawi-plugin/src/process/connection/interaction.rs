use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct Interaction {
    ids: Mutex<BTreeSet<u64>>,
    started: Mutex<Option<Instant>>,
    paused: Mutex<Duration>,
}

impl Interaction {
    pub(crate) fn begin(&self, id: u64) {
        let mut ids = self.ids.lock().unwrap_or_else(|error| error.into_inner());
        if ids.is_empty() {
            *self
                .started
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = Some(Instant::now());
        }
        ids.insert(id);
    }

    pub(crate) fn end(&self, id: u64) {
        let mut ids = self.ids.lock().unwrap_or_else(|error| error.into_inner());
        if !ids.remove(&id) || !ids.is_empty() {
            return;
        }
        let started = self
            .started
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(started) = started {
            *self
                .paused
                .lock()
                .unwrap_or_else(|error| error.into_inner()) += started.elapsed();
        }
    }

    pub(crate) fn reset(&self) {
        self.ids
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clear();
        *self
            .started
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
        *self
            .paused
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Duration::ZERO;
    }

    pub(crate) fn remaining(&self, started: Instant, timeout: Duration) -> Duration {
        let current = self
            .started
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .map_or(Duration::ZERO, |paused_at| paused_at.elapsed());
        let paused = *self
            .paused
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        (started + timeout + paused + current).saturating_duration_since(Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_wait_extends_the_deadline() {
        let interaction = Interaction::default();
        let started = Instant::now();
        let timeout = Duration::from_millis(100);
        std::thread::sleep(Duration::from_millis(20));
        interaction.begin(7);
        std::thread::sleep(Duration::from_millis(40));
        assert!(interaction.remaining(started, timeout) > Duration::from_millis(60));
        interaction.end(7);
        assert!(interaction.remaining(started, timeout) > Duration::from_millis(60));
    }

    #[test]
    fn reset_restores_the_remaining_timeout() {
        let interaction = Interaction::default();
        let started = Instant::now();
        let timeout = Duration::from_millis(50);
        interaction.begin(1);
        std::thread::sleep(Duration::from_millis(20));
        interaction.reset();
        assert!(interaction.remaining(started, timeout) > Duration::from_millis(20));
    }
}
