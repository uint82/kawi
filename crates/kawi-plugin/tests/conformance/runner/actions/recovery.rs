use serde_json::{json, Value};

use kawi_plugin::{PluginError, PluginStatus};

use super::super::super::harness::{ConformanceHost, Runtime};
use super::super::super::normalize::{normalize_result, Outcome};
use super::super::super::scenario::Scenario;

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
