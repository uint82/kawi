use std::net::Shutdown;
use std::thread;

use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::super::harness::ConformanceHost;
use super::super::super::super::scenario::NetworkTarget;
use super::{
    accept, assert_healthy_after_cap, assert_received, drain, parse_count, send_bytes,
    CONNECTION_BYTES, WRITE_TIMEOUT,
};

pub(super) fn run(host: &ConformanceHost, target: NetworkTarget) -> Result<Value, PluginError> {
    let request = host.start_invocation("splice-cap", json!([host.network_target(target)]))?;
    let mut source = accept(host)?;
    let mut spare = accept(host)?;
    let mut destination = accept(host)?;
    spare
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .map_err(|error| PluginError::Protocol(format!("set write timeout: {error}")))?;
    let sent_spare = send_bytes(&mut spare, 1)?;
    if sent_spare != 1 {
        return Err(PluginError::Protocol(format!(
            "sent {sent_spare} spare bytes, expected 1"
        )));
    }
    source
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .map_err(|error| PluginError::Protocol(format!("set write timeout: {error}")))?;
    let writer = thread::spawn(move || {
        let sent = send_bytes(&mut source, CONNECTION_BYTES as usize + 1);
        let _ = source.shutdown(Shutdown::Write);
        sent
    });
    let received = drain(&mut destination)?;
    let sent = writer
        .join()
        .map_err(|_| PluginError::Protocol("splice writer panicked".to_string()))??;
    if sent < CONNECTION_BYTES as usize {
        return Err(PluginError::Protocol(format!(
            "sent {sent} bytes, expected at least {CONNECTION_BYTES}"
        )));
    }
    let result = host.wait_for_finished(request)?;
    assert_received(received, CONNECTION_BYTES)?;
    assert_splice(&result)?;
    assert_healthy_after_cap(host)?;
    Ok(result)
}

fn assert_splice(result: &Value) -> Result<(), PluginError> {
    let text = result
        .as_str()
        .ok_or_else(|| PluginError::Protocol("splice cap returned no text".to_string()))?;
    let mut parts = text.splitn(3, ':');
    let prefix = parse_count(parts.next(), "splice prefix")?;
    let straddle = parse_count(parts.next(), "splice straddle")?;
    let outcome = parts.next().ok_or_else(|| {
        PluginError::Protocol(format!("splice cap returned unexpected text: {text}"))
    })?;
    if prefix != CONNECTION_BYTES - 1 {
        return Err(PluginError::Protocol(format!(
            "splice prefix wrote {prefix} bytes, expected {}",
            CONNECTION_BYTES - 1
        )));
    }
    if straddle != 1 {
        return Err(PluginError::Protocol(format!(
            "straddle splice returned {straddle}, expected 1"
        )));
    }
    if outcome != "closed" {
        return Err(PluginError::Protocol(format!(
            "splice after the cap ended with {outcome}, expected closed"
        )));
    }
    Ok(())
}
