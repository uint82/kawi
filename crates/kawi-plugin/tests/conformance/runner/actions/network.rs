use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::harness::{ConformanceHost, Runtime};
use super::super::super::scenario::{network_case, Scenario};

pub(crate) fn run_network_probe(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    assert_eq!(runtime, Runtime::Wasm);
    let case = network_case(scenario.id);
    host.invoke(case.command, json!([host.network_target(case.target)]))
}
