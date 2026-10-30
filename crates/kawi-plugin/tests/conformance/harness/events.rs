use serde_json::{json, Value};

use kawi_plugin::NotifyOutcome;

use super::host::ConformanceHost;
use super::model::Runtime;
use super::PLUGIN_NAME;

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

    pub(crate) fn block_queued_events(&self) {
        if self.runtime == Runtime::Process {
            self.inject_pipe_block();
        }
    }

    pub(crate) fn release_events(&self) {
        self.release_host_responses();
    }

    fn inject_pipe_block(&self) {
        self.host
            .respond(
                PLUGIN_NAME,
                json!(999),
                Ok(json!({ "pad": "x".repeat(1024 * 1024) })),
            )
            .expect("inject pipe block");
    }
}
