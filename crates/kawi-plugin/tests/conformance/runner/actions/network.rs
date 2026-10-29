use std::io::Read;
use std::time::Duration;

use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::harness::{ConformanceHost, Runtime};
use super::super::super::scenario::{network_case, NetworkTarget, Scenario};

pub(crate) fn run_network_probe(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    assert_eq!(runtime, Runtime::Wasm);
    let case = network_case(scenario.id);
    if case.command == "hold" {
        return run_restart_close(host);
    }
    host.invoke(case.command, json!([host.network_target(case.target)]))
}

fn run_restart_close(host: &ConformanceHost) -> Result<Value, PluginError> {
    let target = host.network_target(NetworkTarget::Allowed);
    let held = host.invoke("hold", json!([target]))?;
    let mut stream = host
        .accept_allowed()
        .map_err(|error| PluginError::Protocol(format!("accept held socket: {error}")))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| PluginError::Protocol(format!("set read timeout: {error}")))?;
    host.restart();
    host.invoke("ping", json!(null))?;
    let mut byte = [0_u8; 1];
    match stream.read(&mut byte) {
        Ok(0) => Ok(held),
        Ok(_) => Err(PluginError::Protocol(
            "socket stayed open after restart".to_string(),
        )),
        Err(error) => Err(PluginError::Protocol(format!(
            "read after restart: {error}"
        ))),
    }
}
