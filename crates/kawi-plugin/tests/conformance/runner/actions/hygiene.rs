use serde_json::{json, Value};

use kawi_plugin::{sanitize, PluginError, INVALID_POSITION};

use super::super::super::harness::{ConformanceHost, Runtime};
use super::super::super::scenario::Scenario;

pub(crate) fn run_control_text(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    let result = host.invoke(runtime.command(scenario.behavior), json!(null))?;
    let message = host
        .last_host_request("ui.message")
        .and_then(|params| params.get("message").cloned())
        .and_then(|message| message.as_str().map(str::to_string))
        .expect("control text must send ui.message");
    assert_eq!(
        sanitize(&message),
        "safered next ",
        "{runtime:?} {}: ui.message was not sanitized",
        scenario.id
    );
    let params = host
        .last_host_request("editor.set_diagnostics")
        .expect("control text must publish diagnostics");
    let diagnostic = &params["diagnostics"][0];
    assert_eq!(
        sanitize(diagnostic["message"].as_str().unwrap()),
        "badtext line",
        "{runtime:?} {}: diagnostic message was not sanitized",
        scenario.id
    );
    assert_eq!(
        sanitize(diagnostic["code"].as_str().unwrap()),
        "E1",
        "{runtime:?} {}: diagnostic code was not sanitized",
        scenario.id
    );
    Ok(result)
}

pub(crate) fn run_mid_codepoint(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    let result = host.invoke(runtime.command(scenario.behavior), json!(null));
    match &result {
        Err(PluginError::Rpc(error)) if error.code == INVALID_POSITION => {
            assert!(
                error.message.contains("codepoint"),
                "{runtime:?} {}: wrong invalid-position reason: {}",
                scenario.id,
                error.message
            );
        }
        _ => panic!(
            "{runtime:?} {}: expected INVALID_POSITION, observed {result:?}",
            scenario.id
        ),
    }
    let observation = host.observation();
    assert_eq!(
        observation.applied_edits, 0,
        "{runtime:?} {}: invalid edit was applied",
        scenario.id
    );
    assert_eq!(
        observation.buffer, "é\n",
        "{runtime:?} {}: buffer changed after invalid edit",
        scenario.id
    );
    result
}
