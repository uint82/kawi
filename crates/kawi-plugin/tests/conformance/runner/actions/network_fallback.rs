use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::harness::{ConformanceHost, Runtime};
use super::super::super::scenario::{NetworkTarget, Scenario};

pub(crate) fn run_network_scripted(
    host: &ConformanceHost,
    runtime: Runtime,
    _scenario: &Scenario,
) -> Result<Value, PluginError> {
    assert_eq!(runtime, Runtime::Wasm);
    host.invoke(
        "resolve-list",
        json!([host.network_target(NetworkTarget::Hostname)]),
    )
}
