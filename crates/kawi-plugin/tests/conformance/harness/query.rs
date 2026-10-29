use std::thread;
use std::time::{Duration, Instant};

use kawi_plugin::PluginStatus;

use super::host::ConformanceHost;
use super::model::Observation;
use super::{START_TIMEOUT, STATUS_POLL};

impl ConformanceHost {
    pub(crate) fn status(&self) -> Option<PluginStatus> {
        self.host.status(super::PLUGIN_NAME)
    }

    pub(crate) fn wait_backoff(&self) {
        if let Some(PluginStatus::Backoff { remaining_ms }) = self.status() {
            thread::sleep(Duration::from_millis(remaining_ms.saturating_add(10)));
        }
    }

    pub(crate) fn wait_for_host_request(&self, method: &str) -> bool {
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            if self
                .observation()
                .host_requests
                .iter()
                .any(|request| request.method == method)
            {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(STATUS_POLL);
        }
    }

    pub(crate) fn wait_for_host_response(&self, method: &str) -> bool {
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            if self
                .observation()
                .host_requests
                .iter()
                .any(|request| request.method == method && request.responded)
            {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(STATUS_POLL);
        }
    }

    pub(crate) fn observation(&self) -> Observation {
        self.observation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(crate) fn host_request_count(&self) -> usize {
        self.observation().host_requests.len()
    }
}
