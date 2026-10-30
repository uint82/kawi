use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::super::harness::ConformanceHost;
use super::super::super::super::scenario::NetworkTarget;
use super::{
    assert_plugin_ready, assert_received, drain_allowed, parse_count, CONNECTION_BYTES,
    SESSION_BYTES,
};

const SESSION_CONNECTIONS: usize = 9;

pub(super) fn run(host: &ConformanceHost, target: NetworkTarget) -> Result<Value, PluginError> {
    let request = host.start_invocation("write-session", json!([host.network_target(target)]))?;
    let mut received = 0_u64;
    for _ in 0..SESSION_CONNECTIONS {
        received += drain_allowed(host)?;
    }
    let result = host.wait_for_finished(request)?;
    assert_received(received, SESSION_BYTES)?;
    assert_session(&result)?;
    assert_plugin_ready(host)?;
    Ok(result)
}

fn assert_session(result: &Value) -> Result<(), PluginError> {
    let text = result
        .as_str()
        .ok_or_else(|| PluginError::Protocol("session write returned no text".to_string()))?;
    let entries = text.split(',').collect::<Vec<_>>();
    if entries.len() != SESSION_CONNECTIONS {
        return Err(PluginError::Protocol(format!(
            "session write returned {} entries, expected {SESSION_CONNECTIONS}",
            entries.len()
        )));
    }
    for (index, entry) in entries.iter().enumerate() {
        let (written, outcome) = parse_entry(entry, index)?;
        let expected = expected_written(index);
        if written != expected {
            return Err(PluginError::Protocol(format!(
                "session connection {index} wrote {written} bytes, expected {expected}: {outcome}"
            )));
        }
    }
    Ok(())
}

fn expected_written(index: usize) -> u64 {
    if index + 1 == SESSION_CONNECTIONS {
        0
    } else {
        CONNECTION_BYTES
    }
}

fn parse_entry(entry: &str, index: usize) -> Result<(u64, &str), PluginError> {
    let (written, outcome) = entry.split_once(':').ok_or_else(|| {
        PluginError::Protocol(format!(
            "session connection {index} returned unexpected text"
        ))
    })?;
    let written = parse_count(Some(written), "session write")?;
    if index + 1 == SESSION_CONNECTIONS && outcome == "none" {
        return Err(PluginError::Protocol(
            "session cap was not observed on the final connection".to_string(),
        ));
    }
    Ok((written, outcome))
}
