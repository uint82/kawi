use serde_json::Value;

use kawi_plugin::NotifyOutcome;

use super::host::ConformanceHost;

impl ConformanceHost {
    pub(crate) fn stall_events(&self, event: &str, payload: Value) {
        self.withhold_host_responses();
        assert_eq!(
            self.notify(event, payload),
            NotifyOutcome::Queued,
            "{}: {event} was not queued",
            self.label()
        );
        assert!(
            self.wait_for_host_request("ui.message"),
            "{}: {event} was not delivered before revocation",
            self.label()
        );
    }

    pub(crate) fn release_events(&self) {
        self.release_host_responses();
    }
}
