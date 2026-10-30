use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::super::harness::ConformanceHost;
use super::super::super::super::scenario::NetworkTarget;
use super::{
    accept, assert_healthy_after_cap, assert_received, drain_allowed, parse_count, parse_counted,
    CONNECTION_BYTES,
};

pub(super) fn run(
    host: &ConformanceHost,
    target: NetworkTarget,
    command: &str,
) -> Result<Value, PluginError> {
    let request = host.start_invocation(command, json!([host.network_target(target)]))?;
    let received = drain_allowed(host)?;
    let result = host.wait_for_finished(request)?;
    assert_outbound_cap(received, &result)?;
    assert_healthy_after_cap(host)?;
    Ok(result)
}

pub(super) fn run_no_reader(
    host: &ConformanceHost,
    target: NetworkTarget,
) -> Result<Value, PluginError> {
    let request = host.start_invocation("write-strict", json!([host.network_target(target)]))?;
    let _stream = accept(host)?;
    host.wait_for_finished(request)
}

pub(super) fn run_straddle(
    host: &ConformanceHost,
    target: NetworkTarget,
) -> Result<Value, PluginError> {
    let request = host.start_invocation("write-straddle", json!([host.network_target(target)]))?;
    let received = drain_allowed(host)?;
    let result = host.wait_for_finished(request)?;
    assert_received(received, CONNECTION_BYTES)?;
    let (prefix, straddle, outcome) = parse_straddle(&result)?;
    assert_straddle(prefix, straddle, outcome)?;
    assert_healthy_after_cap(host)?;
    Ok(result)
}

fn assert_outbound_cap(received: u64, result: &Value) -> Result<(), PluginError> {
    assert_received(received, CONNECTION_BYTES)?;
    let (written, outcome) = parse_counted(result, "capped write")?;
    assert_written(written, outcome)?;
    assert_cap_observed(outcome)
}

fn parse_straddle(result: &Value) -> Result<(u64, u64, &str), PluginError> {
    let text = result
        .as_str()
        .ok_or_else(|| PluginError::Protocol("straddle write returned no text".to_string()))?;
    let mut parts = text.splitn(3, ':');
    let prefix = parse_count(parts.next(), "straddle prefix")?;
    let straddle = parse_count(parts.next(), "straddle write")?;
    let outcome = parts.next().ok_or_else(|| {
        PluginError::Protocol(format!("straddle write returned unexpected text: {text}"))
    })?;
    Ok((prefix, straddle, outcome))
}

fn assert_straddle(prefix: u64, straddle: u64, outcome: &str) -> Result<(), PluginError> {
    if prefix != CONNECTION_BYTES - 1 {
        return Err(PluginError::Protocol(format!(
            "straddle prefix wrote {prefix} bytes, expected {}",
            CONNECTION_BYTES - 1
        )));
    }
    if straddle != 1 {
        return Err(PluginError::Protocol(format!(
            "straddle write returned {straddle}, expected 1"
        )));
    }
    if outcome.starts_with("ok:") {
        return Err(PluginError::Protocol(format!(
            "write after the cap succeeded: {outcome}"
        )));
    }
    Ok(())
}

fn assert_written(written: u64, outcome: &str) -> Result<(), PluginError> {
    if written == CONNECTION_BYTES {
        return Ok(());
    }
    Err(PluginError::Protocol(format!(
        "guest wrote {written} bytes, expected {CONNECTION_BYTES}: {outcome}"
    )))
}

fn assert_cap_observed(outcome: &str) -> Result<(), PluginError> {
    if outcome != "none" {
        return Ok(());
    }
    Err(PluginError::Protocol(
        "guest did not observe the outbound cap".to_string(),
    ))
}
