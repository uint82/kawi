use serde_json::{json, Value};

use kawi_plugin::{NotifyOutcome, PluginError, PluginStatus};

use super::super::harness::{ConformanceHost, Runtime};
use super::super::normalize::{normalize_result, Outcome};
use super::super::scenario::Scenario;

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

pub(crate) fn run_trap_then_recover(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    let first = host.invoke(scenario.behavior.trap_command(runtime), json!(null));
    assert_eq!(
        normalize_result(&first),
        Outcome::Crashed,
        "{runtime:?} {}: first invocation did not crash: {first:?}",
        scenario.id
    );
    host.wait_backoff();
    let recovered = host.invoke(runtime.command(scenario.behavior), json!(null));
    assert_eq!(
        normalize_result(&recovered),
        Outcome::Success,
        "{runtime:?} {}: recovery invocation failed: {recovered:?}",
        scenario.id
    );
    recovered
}

pub(crate) fn run_five_strike(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    let mut last = Ok(Value::Null);
    for attempt in 0..5 {
        last = host.invoke(scenario.behavior.trap_command(runtime), json!(null));
        assert_eq!(
            normalize_result(&last),
            Outcome::Crashed,
            "{runtime:?} {}: attempt {attempt} did not crash: {last:?}",
            scenario.id
        );
        host.wait_backoff();
    }
    assert_eq!(
        host.status(),
        Some(PluginStatus::Disabled),
        "{runtime:?} {}: plugin was not disabled after five crashes",
        scenario.id
    );
    last
}

pub(crate) fn run_counter_reset(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    let first = host.invoke(scenario.behavior.trap_command(runtime), json!(null));
    assert_eq!(
        normalize_result(&first),
        Outcome::Crashed,
        "{runtime:?} {}: first invocation did not crash: {first:?}",
        scenario.id
    );
    host.wait_backoff();
    let second = host.invoke(runtime.command(scenario.behavior), json!(null));
    assert_eq!(
        normalize_result(&second),
        Outcome::Success,
        "{runtime:?} {}: recovery invocation failed: {second:?}",
        scenario.id
    );
    let third = host.invoke(runtime.command(scenario.behavior), json!(null));
    assert_eq!(
        normalize_result(&third),
        Outcome::Success,
        "{runtime:?} {}: invocation after recovery failed: {third:?}",
        scenario.id
    );
    assert_eq!(
        host.status(),
        Some(PluginStatus::Ready),
        "{runtime:?} {}: success did not reset the crash counter",
        scenario.id
    );
    third
}

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

pub(crate) fn run_restart_with_pending_host_call(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    host.activate_ready();
    host.withhold_host_responses();
    let request = host
        .start_invocation(runtime.command(scenario.behavior), json!(null))
        .expect("invoke conformance plugin");
    assert!(
        host.wait_for_host_request("editor.get_state"),
        "{runtime:?} {}: plugin never issued its host call",
        scenario.id
    );
    host.restart();
    let first = host.wait_for_finished(request);
    host.release_host_responses();
    assert_eq!(
        normalize_result(&first),
        Outcome::Crashed,
        "{runtime:?} {}: restart did not cancel the pending call: {first:?}",
        scenario.id
    );
    host.wait_backoff();
    host.invoke(runtime.command(scenario.behavior), json!(null))
}

pub(crate) fn event_payload(event: &str) -> Value {
    match event {
        "buffer_changed" => json!({
            "buffer": 1,
            "revision": 2,
            "previous_revision": 1,
            "origin": "user",
            "full_resync": false,
            "changes": [],
        }),
        "buffer_opened" => json!({
            "buffer": 1,
            "path": null,
            "language": null,
            "revision": 1,
        }),
        "file_opened" => json!({
            "buffer": 1,
            "path": "/tmp/conformance.rs",
            "language": "rust",
            "revision": 1,
        }),
        "buffer_closed" => json!({
            "buffer": 1,
            "path": null,
        }),
        "file_saved" => json!({
            "buffer": 1,
            "path": null,
            "revision": 1,
        }),
        "mode_changed" => json!({
            "buffer": 1,
            "from": "normal",
            "to": "insert",
        }),
        "diagnostics_changed" => json!({
            "buffer": 1,
            "path": "/tmp/conformance.rs",
            "revision": 1,
            "diagnostics": [],
        }),
        _ => panic!("unknown event: {event}"),
    }
}
