use std::thread;

use serde_json::{json, Value};

use super::expectations::{
    parse, validate, Expectation, ExpectationFile, PROCESS_EXPECTATIONS, WASM_EXPECTATIONS,
};
use super::harness::{ConformanceHost, Runtime};
use super::normalize::{normalize_result, Outcome};
use super::report::Report;
use super::scenario::{Action, Expected, Scenario, SCENARIOS};

struct RowResult {
    runtime: Runtime,
    scenario: &'static Scenario,
    result: Result<(), String>,
}

pub(super) fn run() -> Report {
    let process = parse("process", PROCESS_EXPECTATIONS);
    let wasm = parse("wasm", WASM_EXPECTATIONS);
    assert_expectations_valid(&process);
    assert_expectations_valid(&wasm);
    let runtimes = enabled_runtimes();
    let results = run_rows(&runtimes, &process, &wasm);
    let mut report = Report::default();
    for row in results {
        let runtime = report.runtime(row.runtime);
        match row.result {
            Ok(()) => runtime.record_pass(),
            Err(failure) => runtime.record_failure(format!(
                "{} [{}]: {failure}",
                row.scenario.id,
                row.scenario.group.label()
            )),
        }
    }
    record_not_supported(&mut report, &process, &wasm, &runtimes);
    report
}

fn assert_expectations_valid(file: &ExpectationFile) {
    let problems = validate(SCENARIOS, file);
    assert!(problems.is_empty(), "{problems:#?}");
}

fn enabled_runtimes() -> Vec<Runtime> {
    let mut runtimes = vec![Runtime::Process];
    if cfg!(feature = "wasm") {
        runtimes.push(Runtime::Wasm);
    }
    runtimes
}

fn run_rows(
    runtimes: &[Runtime],
    process: &ExpectationFile,
    wasm: &ExpectationFile,
) -> Vec<RowResult> {
    let rows = runtimes
        .iter()
        .flat_map(|runtime| {
            SCENARIOS
                .iter()
                .filter(move |scenario| is_pass(*runtime, process, wasm, scenario))
                .map(move |scenario| (*runtime, scenario))
        })
        .collect::<Vec<_>>();
    thread::scope(|scope| {
        let handles = rows
            .into_iter()
            .map(|(runtime, scenario)| {
                scope.spawn(move || RowResult {
                    runtime,
                    scenario,
                    result: run_row(runtime, scenario),
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("conformance row panicked"))
            .collect()
    })
}

fn is_pass(
    runtime: Runtime,
    process: &ExpectationFile,
    wasm: &ExpectationFile,
    scenario: &Scenario,
) -> bool {
    expectation(runtime, process, wasm, scenario).is_pass()
}

fn expectation<'a>(
    runtime: Runtime,
    process: &'a ExpectationFile,
    wasm: &'a ExpectationFile,
    scenario: &Scenario,
) -> &'a Expectation {
    let file = match runtime {
        Runtime::Process => process,
        Runtime::Wasm => wasm,
    };
    file.rows
        .get(scenario.id)
        .unwrap_or_else(|| panic!("{}: missing {} status", scenario.id, runtime.label()))
}

fn run_row(runtime: Runtime, scenario: &'static Scenario) -> Result<(), String> {
    let host = ConformanceHost::start(runtime, scenario);
    let command = runtime.command(scenario.behavior);
    let result = match scenario.action {
        Action::Invoke => host.invoke(command, json!(null)),
        Action::RevokeThenInvoke => {
            host.activate_ready();
            host.revoke_all();
            host.invoke(command, json!(null))
        }
        Action::RevokeThenNotify => {
            host.activate_ready();
            host.revoke_all();
            let event = scenario
                .behavior
                .event()
                .expect("revoke action needs an event behavior");
            let outcome = host.notify(event, event_payload(event));
            return assert_revoked(outcome);
        }
        Action::NotifyThenInvoke => {
            host.activate_ready();
            let event = scenario
                .behavior
                .event()
                .expect("notify action needs an event behavior");
            assert_eq!(
                host.notify(event, event_payload(event)),
                kawi_plugin::NotifyOutcome::Queued,
                "{event} was not queued"
            );
            assert!(
                host.wait_for_host_response("ui.message"),
                "{event} was not delivered"
            );
            host.invoke(command, json!(null))
        }
        Action::StaleRevision => {
            host.set_stale_revision(true);
            host.invoke(command, json!(null))
        }
        Action::Trap => host.invoke(scenario.behavior.trap_command(runtime), json!(null)),
        Action::TrapThenRecover => run_trap_then_recover(&host, runtime, scenario),
        Action::FiveStrike => run_five_strike(&host, runtime, scenario),
        Action::CounterReset => run_counter_reset(&host, runtime, scenario),
        Action::RestartWithPendingHostCall => {
            run_restart_with_pending_host_call(&host, runtime, scenario)
        }
        Action::QueuedRevocation => run_queued_revocation(&host, runtime, scenario),
        Action::InvalidPosition => {
            host.set_invalid_position(true);
            host.invoke(command, json!(null))
        }
    };
    let observed = normalize_result(&result);
    let expected = expected_outcome(scenario.expected);
    if observed != expected {
        return Err(format!(
            "expected {expected:?}, observed {observed:?} ({result:?}); {}",
            host.observation().logs.join(" | ")
        ));
    }
    assert_denial_was_enforced(&host, scenario, &result)
}

fn assert_denial_was_enforced(
    host: &ConformanceHost,
    scenario: &Scenario,
    result: &Result<serde_json::Value, kawi_plugin::PluginError>,
) -> Result<(), String> {
    let Expected::Denied(capability) = scenario.expected else {
        return Ok(());
    };
    if !result.is_err() {
        return Err(format!("denial {capability} returned a successful result"));
    }
    let observation = host.observation();
    let denied = observation
        .host_requests
        .iter()
        .any(|request| request.denied && request.method == "editor.get_state");
    if denied {
        Ok(())
    } else {
        Err(format!(
            "host never denied editor.get_state for {capability}"
        ))
    }
}

fn assert_revoked(outcome: kawi_plugin::NotifyOutcome) -> Result<(), String> {
    if outcome == kawi_plugin::NotifyOutcome::Denied {
        Ok(())
    } else {
        Err(format!("expected Denied, observed {outcome:?}"))
    }
}

fn expected_outcome(expected: Expected) -> Outcome {
    match expected {
        Expected::Success => Outcome::Success,
        Expected::Denied(capability) => Outcome::denied(capability),
        Expected::Protocol => Outcome::Protocol,
        Expected::Crashed => Outcome::Crashed,
        Expected::Timeout => Outcome::Timeout,
    }
}

fn run_trap_then_recover(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, kawi_plugin::PluginError> {
    let first = host.invoke(scenario.behavior.trap_command(runtime), json!(null));
    assert_eq!(
        normalize_result(&first),
        Outcome::Crashed,
        "first invocation did not crash: {first:?}"
    );
    host.wait_backoff();
    host.invoke(runtime.command(scenario.behavior), json!(null))
}

fn run_five_strike(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, kawi_plugin::PluginError> {
    let mut last = Ok(Value::Null);
    for attempt in 0..5 {
        last = host.invoke(scenario.behavior.trap_command(runtime), json!(null));
        assert_eq!(
            normalize_result(&last),
            Outcome::Crashed,
            "attempt {attempt} did not crash: {last:?}"
        );
        host.wait_backoff();
    }
    assert_eq!(
        host.status(),
        Some(kawi_plugin::PluginStatus::Disabled),
        "plugin was not disabled after five crashes"
    );
    last
}

fn run_counter_reset(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, kawi_plugin::PluginError> {
    let first = host.invoke(scenario.behavior.trap_command(runtime), json!(null));
    assert_eq!(normalize_result(&first), Outcome::Crashed);
    host.wait_backoff();
    let second = host.invoke(runtime.command(scenario.behavior), json!(null));
    assert_eq!(normalize_result(&second), Outcome::Success);
    let third = host.invoke(runtime.command(scenario.behavior), json!(null));
    assert_eq!(normalize_result(&third), Outcome::Success);
    assert_eq!(
        host.status(),
        Some(kawi_plugin::PluginStatus::Ready),
        "success did not reset the crash counter"
    );
    third
}

fn run_queued_revocation(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, kawi_plugin::PluginError> {
    host.activate_ready();
    let event = scenario
        .behavior
        .event()
        .expect("queued revocation needs an event behavior");
    host.stall_events(event, event_payload(event));
    for _ in 0..16 {
        assert_eq!(
            host.notify(event, event_payload(event)),
            kawi_plugin::NotifyOutcome::Queued,
            "{event} was not queued"
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
        "host requests after revocation: {observed_after:?}"
    );
    result
}

fn run_restart_with_pending_host_call(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, kawi_plugin::PluginError> {
    host.activate_ready();
    host.withhold_host_responses();
    let request = host.start_invocation(runtime.command(scenario.behavior), json!(null));
    assert!(
        host.wait_for_host_request("editor.get_state"),
        "plugin never issued its host call"
    );
    host.restart();
    let first = host.wait_for_finished(request);
    host.release_host_responses();
    assert_eq!(
        normalize_result(&first),
        Outcome::Crashed,
        "restart did not cancel the pending call: {first:?}"
    );
    host.wait_backoff();
    host.invoke(runtime.command(scenario.behavior), json!(null))
}

fn event_payload(event: &str) -> Value {
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

fn record_not_supported(
    report: &mut Report,
    process: &ExpectationFile,
    wasm: &ExpectationFile,
    runtimes: &[Runtime],
) {
    for scenario in SCENARIOS {
        for runtime in runtimes {
            let expectation = expectation(*runtime, process, wasm, scenario);
            if let Expectation::NotSupported { kind, .. } = expectation {
                report.runtime(*runtime).record_not_supported(kind);
            }
        }
    }
}
