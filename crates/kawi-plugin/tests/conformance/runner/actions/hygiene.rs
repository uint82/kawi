use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::harness::{ConformanceHost, Runtime};
use super::super::super::scenario::Scenario;

pub(crate) fn run_control_text(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    let result = host.invoke(runtime.command(scenario.behavior), json!(null))?;
    assert_eq!(
        result,
        json!("a\n\t\u{1}b"),
        "{runtime:?} {}: control text changed in transit",
        scenario.id
    );
    Ok(result)
}
