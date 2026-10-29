use serde_json::{json, Value};

use kawi_plugin::{NotifyOutcome, PluginError};

use super::super::super::harness::{ConformanceHost, Runtime};
use super::super::super::normalize::{normalize_result, Outcome};
use super::super::super::scenario::Scenario;
use super::events::event_payload;

pub(crate) fn run_oversize_invocation(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    let oversized = json!({
        "pad": "x".repeat(kawi_jsonrpc::framing::MAX_MESSAGE_BYTES + 1),
    });
    let first = host.invoke(runtime.command(scenario.behavior), oversized);
    assert_eq!(
        normalize_result(&first),
        Outcome::Protocol,
        "{runtime:?} {}: oversize invocation did not fail: {first:?}",
        scenario.id
    );
    host.invoke(runtime.command(scenario.behavior), json!(null))
}

pub(crate) fn run_oversize_event(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    host.activate_ready();
    let event = scenario
        .behavior
        .event()
        .expect("oversize event needs an event behavior");
    let mut payload = event_payload(event);
    payload["pad"] = json!("x".repeat(kawi_jsonrpc::framing::MAX_MESSAGE_BYTES + 1));
    assert_eq!(
        host.notify(event, payload),
        NotifyOutcome::Overflow,
        "{runtime:?} {}: oversize event was not dropped",
        scenario.id
    );
    let result = host.invoke(runtime.command(scenario.behavior), json!(null));
    let observation = host.observation();
    assert!(
        observation
            .host_requests
            .iter()
            .all(|request| request.method != "ui.message"),
        "{runtime:?} {}: oversize event reached the plugin",
        scenario.id
    );
    result
}

pub(crate) fn run_oversize_diagnostics(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    let first = host.invoke(runtime.command(scenario.behavior), json!(null));
    assert_eq!(
        normalize_result(&first),
        Outcome::Protocol,
        "{runtime:?} {}: oversize diagnostics did not fail: {first:?}",
        scenario.id
    );
    host.wait_backoff();
    host.invoke("ping", json!(null))
}

pub(crate) fn run_oversize_host_response(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    host.set_oversize_response(true);
    let first = host.invoke(runtime.command(scenario.behavior), json!(null));
    assert_eq!(
        normalize_result(&first),
        Outcome::Protocol,
        "{runtime:?} {}: oversize host response did not fail: {first:?}",
        scenario.id
    );
    host.set_oversize_response(false);
    host.wait_backoff();
    host.invoke(runtime.command(scenario.behavior), json!(null))
}
