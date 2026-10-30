use std::net::Shutdown;

use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::super::harness::ConformanceHost;
use super::super::super::super::scenario::NetworkTarget;
use super::{
    accept, assert_healthy_after_cap, parse_count, parse_counted, send_bytes, CONNECTION_BYTES,
    WRITE_TIMEOUT,
};

const SEND_LIMIT: usize = 9 * 1024 * 1024;

pub(super) fn run(host: &ConformanceHost, target: NetworkTarget) -> Result<Value, PluginError> {
    let request = host.start_invocation("read-cap", json!([host.network_target(target)]))?;
    let mut stream = accept(host)?;
    send_inbound(&mut stream)?;
    let result = host.wait_for_finished(request)?;
    assert_inbound_cap(&result)?;
    assert_healthy_after_cap(host)?;
    Ok(result)
}

pub(super) fn run_straddle(
    host: &ConformanceHost,
    target: NetworkTarget,
) -> Result<Value, PluginError> {
    let request = host.start_invocation("read-straddle", json!([host.network_target(target)]))?;
    let mut stream = accept(host)?;
    send_inbound(&mut stream)?;
    let result = host.wait_for_finished(request)?;
    assert_inbound_straddle(&result)?;
    assert_healthy_after_cap(host)?;
    Ok(result)
}

pub(super) fn run_no_sender(
    host: &ConformanceHost,
    target: NetworkTarget,
) -> Result<Value, PluginError> {
    let request = host.start_invocation("read-strict", json!([host.network_target(target)]))?;
    let _stream = accept(host)?;
    host.wait_for_finished(request)
}

fn send_inbound(stream: &mut std::net::TcpStream) -> Result<(), PluginError> {
    stream
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .map_err(|error| PluginError::Protocol(format!("set write timeout: {error}")))?;
    let sent = send_bytes(stream, SEND_LIMIT)?;
    let _ = stream.shutdown(Shutdown::Write);
    if sent < CONNECTION_BYTES as usize {
        return Err(PluginError::Protocol(format!(
            "sent {sent} bytes, expected at least {CONNECTION_BYTES}"
        )));
    }
    Ok(())
}

fn assert_inbound_cap(result: &Value) -> Result<(), PluginError> {
    let (read, outcome) = parse_counted(result, "capped read")?;
    if read != CONNECTION_BYTES {
        return Err(PluginError::Protocol(format!(
            "guest read {read} bytes, expected {CONNECTION_BYTES}"
        )));
    }
    if outcome != "eof" {
        return Err(PluginError::Protocol(format!(
            "guest read ended with {outcome}, expected eof"
        )));
    }
    Ok(())
}

fn assert_inbound_straddle(result: &Value) -> Result<(), PluginError> {
    let text = result
        .as_str()
        .ok_or_else(|| PluginError::Protocol("straddle read returned no text".to_string()))?;
    let mut parts = text.splitn(3, ':');
    let prefix = parse_count(parts.next(), "straddle prefix")?;
    let straddle = parse_count(parts.next(), "straddle read")?;
    let outcome = parts.next().ok_or_else(|| {
        PluginError::Protocol(format!("straddle read returned unexpected text: {text}"))
    })?;
    if prefix != CONNECTION_BYTES - 1 {
        return Err(PluginError::Protocol(format!(
            "straddle prefix read {prefix} bytes, expected {}",
            CONNECTION_BYTES - 1
        )));
    }
    if straddle != 1 {
        return Err(PluginError::Protocol(format!(
            "straddle read returned {straddle}, expected 1"
        )));
    }
    if outcome != "eof" {
        return Err(PluginError::Protocol(format!(
            "read after the cap ended with {outcome}, expected eof"
        )));
    }
    Ok(())
}
