use serde_json::{json, Value};

use kawi_plugin::{NotifyOutcome, PluginError};

use super::super::super::harness::{ConformanceHost, HostRequestRecord, Runtime};
use super::super::super::scenario::Scenario;
use super::events::event_payload;

pub(crate) fn assert_revoked(
    outcome: NotifyOutcome,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<(), String> {
    if outcome == NotifyOutcome::Denied {
        Ok(())
    } else {
        Err(format!(
            "{runtime:?} {}: expected Denied, observed {outcome:?}",
            scenario.id
        ))
    }
}

pub(crate) fn run_queued_revocation(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    host.activate_ready();
    let event = scenario
        .behavior
        .event()
        .expect("queued revocation needs an event behavior");
    host.stall_events(event, event_payload(event));
    for _ in 0..16 {
        assert_eq!(
            host.notify(event, event_payload(event)),
            NotifyOutcome::Queued,
            "{runtime:?} {}: {event} was not queued",
            scenario.id
        );
    }
    host.revoke_everything();
    let revoked_at = host.host_request_count();
    host.release_events();
    let result = host.invoke(runtime.command(scenario.behavior), json!(null));
    let observation = host.observation();
    let observed_after = &observation.host_requests[revoked_at..];
    assert!(
        observed_after.iter().all(|request| request.denied),
        "{runtime:?} {}: allowed host requests after revocation: {observed_after:?}",
        scenario.id
    );
    result
}

pub(crate) fn run_queued_revocation_mid_write(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    host.activate_ready();
    let event = scenario
        .behavior
        .event()
        .expect("queued revocation needs an event behavior");
    host.withhold_apply_edit();
    assert_eq!(
        host.notify(event, event_payload(event)),
        NotifyOutcome::Queued,
        "{runtime:?} {}: {event} was not queued",
        scenario.id
    );
    assert!(
        host.wait_for_host_request("editor.apply_edit"),
        "{runtime:?} {}: plugin never issued its edit",
        scenario.id
    );
    assert_eq!(
        host.notify(event, event_payload(event)),
        NotifyOutcome::Queued,
        "{runtime:?} {}: {event} was not queued",
        scenario.id
    );
    host.revoke_everything();
    let revoked_at = host.host_request_count();
    host.release_host_responses();
    let result = host.invoke("ping", json!(null));
    let observation = host.observation();
    let edits = observation
        .host_requests
        .iter()
        .filter(|request| !request.denied && request.method == "editor.apply_edit")
        .count();
    assert_eq!(
        edits, 1,
        "{runtime:?} {}: in-flight edit was repeated",
        scenario.id
    );
    let starts = observation
        .host_requests
        .iter()
        .filter(|request| !request.denied && is_event_marker(request, "event-write-start"))
        .count();
    assert_eq!(
        starts, 1,
        "{runtime:?} {}: in-flight event was not delivered exactly once",
        scenario.id
    );
    assert!(
        observation.host_requests[revoked_at..]
            .iter()
            .all(|request| request.denied),
        "{runtime:?} {}: queued event reached the revoked host: {:#?}",
        scenario.id,
        observation.host_requests
    );
    result
}

fn is_event_marker(request: &HostRequestRecord, marker: &str) -> bool {
    request.method == "ui.message"
        && request.params.get("message").and_then(Value::as_str) == Some(marker)
}
