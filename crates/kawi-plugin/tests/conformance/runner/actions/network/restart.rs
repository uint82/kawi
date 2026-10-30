use std::io::Read;

use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::super::harness::ConformanceHost;
use super::super::super::super::scenario::NetworkTarget;
use super::accept;

pub(super) fn run(host: &ConformanceHost) -> Result<Value, PluginError> {
    let target = host.network_target(NetworkTarget::Allowed);
    let held = host.invoke("hold", json!([target]))?;
    let mut stream = accept(host)?;
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
