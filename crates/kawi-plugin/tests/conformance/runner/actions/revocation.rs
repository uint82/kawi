use serde_json::{json, Value};

use kawi_plugin::{NotifyOutcome, PluginError};

use super::super::super::harness::{ConformanceHost, Runtime};
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
    host.revoke_all();
    let revoked_at = host.host_request_count();
    host.release_events();
    let result = host.invoke(runtime.command(scenario.behavior), json!(null));
    let observation = host.observation();
    let observed_after = &observation.host_requests[revoked_at..];
    assert!(
        observed_after.iter().all(|request| {
            !matches!(
                request.method.as_str(),
                "ui.message" | "editor.apply_edit" | "editor.set_diagnostics"
            )
        }),
        "{runtime:?} {}: host requests after revocation: {observed_after:?}",
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
    host.withhold_host_responses();
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
    host.block_queued_events();
    assert_eq!(
        host.notify(event, event_payload(event)),
        NotifyOutcome::Queued,
        "{runtime:?} {}: {event} was not queued",
        scenario.id
    );
    host.revoke_all();
    let revoked_at = host.host_request_count();
    host.release_host_responses();
    let result = host.invoke("ping", json!(null));
    let observation = host.observation();
    let edits = observation
        .host_requests
        .iter()
        .filter(|request| request.method == "editor.apply_edit")
        .count();
    assert_eq!(
        edits, 1,
        "{runtime:?} {}: in-flight edit was repeated",
        scenario.id
    );
    assert!(
        observation.host_requests[revoked_at..]
            .iter()
            .all(|request| request.method != "editor.apply_edit"),
        "{runtime:?} {}: edit after revocation",
        scenario.id
    );
    result
}
