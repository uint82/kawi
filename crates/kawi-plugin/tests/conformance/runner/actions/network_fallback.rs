use serde_json::Value;

use kawi_plugin::PluginError;

use super::super::super::harness::{ConformanceHost, Runtime};
use super::super::super::scenario::Scenario;

pub(crate) fn run_network_scripted(
    _host: &ConformanceHost,
    runtime: Runtime,
    _scenario: &Scenario,
) -> Result<Value, PluginError> {
    assert_eq!(runtime, Runtime::Wasm);
    Err(PluginError::Protocol(
        "scripted network rows require the debug-only wasm-test-network feature".to_string(),
    ))
}
