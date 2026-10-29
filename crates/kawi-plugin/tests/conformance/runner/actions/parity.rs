#[cfg(feature = "wasm")]
use serde_json::json;
use serde_json::Value;

use kawi_plugin::PluginError;

use super::super::super::harness::{ConformanceHost, Runtime};
use super::super::super::scenario::Scenario;

#[cfg(feature = "wasm")]
pub(crate) fn run_advertised_parity(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    host.activate_ready();
    let advertised = host.advertised_capabilities();
    let linked = host.linked_capabilities();
    assert_eq!(
        advertised, linked,
        "{runtime:?} {}: advertised capabilities differ from linked imports",
        scenario.id
    );
    host.invoke(runtime.command(scenario.behavior), json!(null))
}

#[cfg(not(feature = "wasm"))]
pub(crate) fn run_advertised_parity(
    _host: &ConformanceHost,
    _runtime: Runtime,
    _scenario: &Scenario,
) -> Result<Value, PluginError> {
    panic!("advertised parity requires the wasm feature")
}
