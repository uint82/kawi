mod actions;

use std::thread;

use serde_json::{json, Value};

use super::expectations::{
    parse, validate, Expectation, ExpectationFile, PROCESS_EXPECTATIONS, WASM_EXPECTATIONS,
};
use super::harness::{ConformanceHost, Runtime};
use super::normalize::{normalize_result, Outcome};
use super::report::Report;
use super::scenario::{scenarios, Action, Expected, Scenario};

use actions::{
    assert_revoked, event_payload, run_advertised_parity, run_control_text, run_counter_reset,
    run_five_strike, run_mid_codepoint, run_network_probe, run_network_scripted,
    run_oversize_diagnostics, run_oversize_event, run_oversize_host_response,
    run_oversize_invocation, run_queued_revocation, run_queued_revocation_mid_write,
    run_restart_with_pending_host_call, run_trap_then_recover,
};

const NORMAL_APPLY_EDITS: usize = 1;
const SINGLE_RETRY_APPLY_EDITS: usize = 2;
const EXHAUSTED_RETRY_APPLY_EDITS: usize = 3;
const EXHAUSTED_RETRY_STATE_CALLS: usize = 3;
const BUFFER_CHANGE_APPLY_EDITS: usize = 1;
const BUFFER_CHANGE_STATE_CALLS: usize = 2;

struct RowResult {
    runtime: Runtime,
    scenario: &'static Scenario,
    result: Result<(), String>,
}

pub(crate) fn run() -> Report {
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
    if !cfg!(feature = "wasm") {
        report.record_not_built(Runtime::Wasm, "wasm feature disabled");
    }
    report.record_sandbox(&format!(
        "{:?} (landlock abi {:?})",
        kawi_process::sandbox_status(),
        kawi_process::sandbox_abi()
    ));
    report
}

fn assert_expectations_valid(file: &ExpectationFile) {
    let problems = validate(scenarios(), file);
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
            scenarios()
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
            return assert_revoked(outcome, runtime, scenario);
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
                "{runtime:?} {}: {event} was not queued",
                scenario.id
            );
            assert!(
                host.wait_for_host_response("ui.message"),
                "{runtime:?} {}: {event} was not delivered",
                scenario.id
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
        Action::QueuedRevocationMidWrite => {
            run_queued_revocation_mid_write(&host, runtime, scenario)
        }
        Action::InvalidPosition => {
            host.set_invalid_position(true);
            host.invoke(command, json!(null))
        }
        Action::OversizeInvocation => run_oversize_invocation(&host, runtime, scenario),
        Action::OversizeEvent => run_oversize_event(&host, runtime, scenario),
        Action::OversizeHostResponse => run_oversize_host_response(&host, runtime, scenario),
        Action::OversizeDiagnostics => run_oversize_diagnostics(&host, runtime, scenario),
        Action::AdvertisedParity => run_advertised_parity(&host, runtime, scenario),
        Action::ControlText => run_control_text(&host, runtime, scenario),
        Action::MidCodepointPosition => run_mid_codepoint(&host, runtime, scenario),
        Action::NetworkProbe => run_network_probe(&host, runtime, scenario),
        Action::NetworkScripted => run_network_scripted(&host, runtime, scenario),
        Action::GatedInvoke => host.invoke(command, json!(null)),
        Action::GatedRevokeThenInvoke => {
            host.activate_ready();
            host.revoke_all();
            host.invoke(command, json!(null))
        }
        Action::Trim => host.invoke(command, json!(null)),
        Action::TrimDenied => {
            host.activate_ready();
            let grants = scenario
                .grants
                .iter()
                .copied()
                .filter(|capability| *capability != kawi_plugin::Capability::EditorWrite)
                .collect::<Vec<_>>();
            host.set_grants(&grants);
            host.invoke(command, json!(null))
        }
        Action::TrimStale => {
            host.set_stale_once();
            host.invoke(command, json!(null))
        }
        Action::TrimStaleExhausted => {
            host.set_stale_revision(true);
            host.invoke(command, json!(null))
        }
        Action::TrimBufferChanged => {
            host.set_stale_once();
            host.set_state_buffer_after_stale(2);
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
    match scenario.action {
        Action::Trim | Action::TrimStale => assert_trim_edits(&host, scenario)?,
        Action::TrimStaleExhausted => assert_trim_failure(
            &host,
            EXHAUSTED_RETRY_APPLY_EDITS,
            EXHAUSTED_RETRY_STATE_CALLS,
        )?,
        Action::TrimBufferChanged => {
            assert_trim_failure(&host, BUFFER_CHANGE_APPLY_EDITS, BUFFER_CHANGE_STATE_CALLS)?
        }
        _ => {}
    }
    assert_denial_was_enforced(&host, scenario, &result)
}

fn assert_trim_edits(host: &ConformanceHost, scenario: &Scenario) -> Result<(), String> {
    let observation = host.observation();
    let requests = observation
        .host_requests
        .iter()
        .filter(|request| request.method == "editor.apply_edit")
        .collect::<Vec<_>>();
    let expected_requests = if scenario.action == Action::TrimStale {
        SINGLE_RETRY_APPLY_EDITS
    } else {
        NORMAL_APPLY_EDITS
    };
    if requests.len() != expected_requests {
        return Err(format!(
            "expected {expected_requests} apply_edit calls, observed {}",
            requests.len()
        ));
    }
    let edits = requests
        .last()
        .and_then(|request| request.params.get("edits"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "apply_edit request is missing edits".to_string())?;
    let expected = serde_json::json!([
        {
            "start": { "line": 0, "byte_column": 2 },
            "end": { "line": 0, "byte_column": 4 },
            "text": "",
        },
        {
            "start": { "line": 1, "byte_column": 4 },
            "end": { "line": 1, "byte_column": 5 },
            "text": "",
        },
    ]);
    if serde_json::Value::Array(edits.clone()) != expected {
        return Err(format!("unexpected trim edits: {edits:?}"));
    }
    Ok(())
}

fn assert_trim_failure(
    host: &ConformanceHost,
    expected_apply_edits: usize,
    expected_state_calls: usize,
) -> Result<(), String> {
    let observation = host.observation();
    let apply_edits = count_requests(&observation, "editor.apply_edit");
    let state_calls = count_requests(&observation, "editor.get_state");
    if apply_edits != expected_apply_edits {
        return Err(format!(
            "expected {expected_apply_edits} apply_edit calls, observed {apply_edits}"
        ));
    }
    if state_calls != expected_state_calls {
        return Err(format!(
            "expected {expected_state_calls} get_state calls, observed {state_calls}"
        ));
    }
    if observation.applied_edits != 0 {
        return Err(format!(
            "expected no applied edits, observed {}",
            observation.applied_edits
        ));
    }
    Ok(())
}

fn count_requests(observation: &super::harness::Observation, method: &str) -> usize {
    observation
        .host_requests
        .iter()
        .filter(|request| request.method == method)
        .count()
}

fn assert_denial_was_enforced(
    host: &ConformanceHost,
    scenario: &Scenario,
    result: &Result<Value, kawi_plugin::PluginError>,
) -> Result<(), String> {
    let Expected::Denied(capability) = scenario.expected else {
        return Ok(());
    };
    if matches!(
        scenario.action,
        Action::NetworkProbe
            | Action::NetworkScripted
            | Action::GatedInvoke
            | Action::GatedRevokeThenInvoke
    ) {
        return Ok(());
    }
    if !result.is_err() {
        return Err(format!("denial {capability} returned a successful result"));
    }
    let method = match capability {
        "editor.read" => "editor.get_state",
        "editor.write" => "editor.apply_edit",
        _ => return Ok(()),
    };
    let observation = host.observation();
    let denied = observation
        .host_requests
        .iter()
        .any(|request| request.denied && request.method == method);
    if denied {
        Ok(())
    } else {
        Err(format!("host never denied {method} for {capability}"))
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

fn record_not_supported(
    report: &mut Report,
    process: &ExpectationFile,
    wasm: &ExpectationFile,
    runtimes: &[Runtime],
) {
    for scenario in scenarios() {
        for runtime in runtimes {
            let expectation = expectation(*runtime, process, wasm, scenario);
            if let Expectation::NotSupported { kind, .. } = expectation {
                report.runtime(*runtime).record_not_supported(kind);
            }
        }
    }
}
