use std::time::{Duration, Instant};

use kawi_plugin::{
    validate_timer_count, validate_timer_delay, validate_timer_interval, validate_timer_payload,
    PluginError, TimerAfterParams, TimerEveryParams, TimerInfo,
};

use super::runtime::PluginRuntime;

pub(super) struct PluginTimer {
    pub(super) plugin: String,
    interval: Option<Duration>,
    payload: Option<String>,
    due: Instant,
}

impl PluginRuntime {
    pub(super) fn register_after_timer(
        &mut self,
        plugin: &str,
        params: TimerAfterParams,
        now: Instant,
    ) -> Result<u64, String> {
        validate_timer_delay(params.delay_ms)?;
        validate_timer_payload(params.payload.as_deref())?;
        self.register_timer(plugin, None, params.payload, now, params.delay_ms)
    }

    pub(super) fn register_every_timer(
        &mut self,
        plugin: &str,
        params: TimerEveryParams,
        now: Instant,
    ) -> Result<u64, String> {
        validate_timer_interval(params.interval_ms)?;
        validate_timer_payload(params.payload.as_deref())?;
        self.register_timer(
            plugin,
            Some(Duration::from_millis(u64::from(params.interval_ms))),
            params.payload,
            now,
            params.interval_ms,
        )
    }

    fn register_timer(
        &mut self,
        plugin: &str,
        interval: Option<Duration>,
        payload: Option<String>,
        now: Instant,
        delay_ms: u32,
    ) -> Result<u64, String> {
        let count = self
            .timers
            .values()
            .filter(|timer| timer.plugin == plugin)
            .count();
        validate_timer_count(count)?;
        self.next_timer_id = self.next_timer_id.wrapping_add(1);
        let id = self.next_timer_id;
        self.timers.insert(
            id,
            PluginTimer {
                plugin: plugin.to_string(),
                interval,
                payload,
                due: now + Duration::from_millis(u64::from(delay_ms)),
            },
        );
        Ok(id)
    }

    pub(super) fn cancel_timer(&mut self, plugin: &str, timer: u64) -> bool {
        if self
            .timers
            .get(&timer)
            .is_some_and(|entry| entry.plugin == plugin)
        {
            self.timers.remove(&timer);
            return true;
        }
        false
    }

    pub(super) fn registered_timers(&self, plugin: &str) -> Vec<TimerInfo> {
        self.timers
            .iter()
            .filter(|(_, timer)| timer.plugin == plugin)
            .map(|(id, timer)| TimerInfo {
                id: *id,
                interval_ms: timer
                    .interval
                    .and_then(|interval| u32::try_from(interval.as_millis()).ok()),
                payload: timer.payload.clone(),
            })
            .collect()
    }

    pub(super) fn clear_plugin_timers(&mut self, plugin: &str) {
        self.timers.retain(|_, timer| timer.plugin != plugin);
    }

    pub(crate) fn timer_deadline(&self, now: Instant) -> Option<Duration> {
        self.timers
            .values()
            .map(|timer| timer.due.saturating_duration_since(now))
            .min()
    }

    pub(super) fn dispatch_due_timers(&mut self, now: Instant) {
        let due = self
            .timers
            .iter()
            .filter(|(_, timer)| timer.due <= now)
            .map(|(id, timer)| (*id, timer.plugin.clone(), timer.payload.clone()))
            .collect::<Vec<_>>();
        for (id, plugin, payload) in due {
            self.advance_timer(id, now);
            let Some(host) = self.host.as_ref() else {
                continue;
            };
            match host.timer_callback(&plugin, id, payload) {
                Ok(kawi_plugin::NotifyOutcome::Overflow) => {
                    self.push_log(&plugin, "event queue full; dropping timer callback");
                }
                Ok(_) => {}
                Err(error) => self.push_log(&plugin, &timer_error(&error)),
            }
        }
    }

    fn advance_timer(&mut self, id: u64, now: Instant) {
        let Some(interval) = self.timers.get(&id).and_then(|timer| timer.interval) else {
            self.timers.remove(&id);
            return;
        };
        if let Some(timer) = self.timers.get_mut(&id) {
            let next = timer.due + interval;
            timer.due = if next <= now { now + interval } else { next };
        }
    }
}

fn timer_error(error: &PluginError) -> String {
    format!("timer callback failed: {error}")
}
