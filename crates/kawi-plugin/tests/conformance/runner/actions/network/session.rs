use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::super::harness::ConformanceHost;
use super::super::super::super::scenario::NetworkTarget;
use super::{
    accept, assert_healthy_after_cap, assert_received, drain_allowed, parse_count, send_bytes,
    CONNECTION_BYTES, SESSION_BYTES, WRITE_TIMEOUT,
};

const SESSION_CONNECTIONS: usize = 9;
const SESSION_HALF: u64 = SESSION_BYTES / 2;
const MIXED_CONNECTIONS: usize = 4;
const SESSION_CLOSE_CONNECTIONS: usize = 16;
const SESSION_CLOSE_BYTES: u64 = 4 * 1024 * 1024;

pub(super) fn run(host: &ConformanceHost, target: NetworkTarget) -> Result<Value, PluginError> {
    let request = host.start_invocation("write-session", json!([host.network_target(target)]))?;
    let mut received = 0_u64;
    for _ in 0..SESSION_CONNECTIONS {
        received += drain_allowed(host)?;
    }
    let result = host.wait_for_finished(request)?;
    assert_received(received, SESSION_BYTES)?;
    assert_session(&result)?;
    assert_healthy_after_cap(host)?;
    Ok(result)
}

pub(super) fn run_mixed(
    host: &ConformanceHost,
    target: NetworkTarget,
) -> Result<Value, PluginError> {
    let request = host.start_invocation("mixed-cap", json!([host.network_target(target)]))?;
    let mut sent = 0_u64;
    for _ in 0..MIXED_CONNECTIONS {
        let mut stream = accept(host)?;
        stream
            .set_write_timeout(Some(WRITE_TIMEOUT))
            .map_err(|error| PluginError::Protocol(format!("set write timeout: {error}")))?;
        let count = send_bytes(&mut stream, CONNECTION_BYTES as usize)?;
        if count < CONNECTION_BYTES as usize {
            return Err(PluginError::Protocol(format!(
                "sent {count} bytes, expected at least {CONNECTION_BYTES}"
            )));
        }
        sent += count as u64;
    }
    let mut received = 0_u64;
    for _ in 0..MIXED_CONNECTIONS {
        received += drain_allowed(host)?;
    }
    received += drain_allowed(host)?;
    let result = host.wait_for_finished(request)?;
    assert_received(received, SESSION_HALF)?;
    assert_received(sent, SESSION_HALF)?;
    assert_mixed(&result)?;
    assert_healthy_after_cap(host)?;
    Ok(result)
}

pub(super) fn run_close(
    host: &ConformanceHost,
    target: NetworkTarget,
) -> Result<Value, PluginError> {
    let request = host.start_invocation("write-close", json!([host.network_target(target)]))?;
    let mut received = 0_u64;
    for _ in 0..=SESSION_CLOSE_CONNECTIONS {
        received += drain_allowed(host)?;
    }
    let result = host.wait_for_finished(request)?;
    assert_received(received, SESSION_BYTES)?;
    assert_close(&result)?;
    assert_healthy_after_cap(host)?;
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
        let (written, outcome) = parse_entry(entry, index, SESSION_CONNECTIONS - 1)?;
        let expected = expected_written(index);
        if written != expected {
            return Err(PluginError::Protocol(format!(
                "session connection {index} wrote {written} bytes, expected {expected}: {outcome}"
            )));
        }
    }
    Ok(())
}

fn assert_close(result: &Value) -> Result<(), PluginError> {
    let text = result
        .as_str()
        .ok_or_else(|| PluginError::Protocol("session close returned no text".to_string()))?;
    let entries = text.split(',').collect::<Vec<_>>();
    if entries.len() != SESSION_CLOSE_CONNECTIONS + 1 {
        return Err(PluginError::Protocol(format!(
            "session close returned {} entries, expected {}",
            entries.len(),
            SESSION_CLOSE_CONNECTIONS + 1
        )));
    }
    for (index, entry) in entries.iter().enumerate() {
        let (written, outcome) = parse_entry(entry, index, SESSION_CLOSE_CONNECTIONS)?;
        let expected = expected_close_written(index);
        if written != expected {
            return Err(PluginError::Protocol(format!(
                "close connection {index} wrote {written} bytes, expected {expected}: {outcome}"
            )));
        }
    }
    Ok(())
}

fn assert_mixed(result: &Value) -> Result<(), PluginError> {
    let text = result
        .as_str()
        .ok_or_else(|| PluginError::Protocol("mixed cap returned no text".to_string()))?;
    let mut parts = text.splitn(3, ':');
    let read = parse_count(parts.next(), "mixed read")?;
    let written = parse_count(parts.next(), "mixed write")?;
    let next_write = parts.next().ok_or_else(|| {
        PluginError::Protocol(format!("mixed cap returned unexpected text: {text}"))
    })?;
    if read != SESSION_HALF {
        return Err(PluginError::Protocol(format!(
            "mixed read {read} bytes, expected {SESSION_HALF}"
        )));
    }
    if written != SESSION_HALF {
        return Err(PluginError::Protocol(format!(
            "mixed write {written} bytes, expected {SESSION_HALF}"
        )));
    }
    if next_write != "closed" {
        return Err(PluginError::Protocol(format!(
            "mixed write after the cap ended with {next_write}, expected closed"
        )));
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

fn expected_close_written(index: usize) -> u64 {
    if index == SESSION_CLOSE_CONNECTIONS {
        0
    } else {
        SESSION_CLOSE_BYTES
    }
}

fn parse_entry(entry: &str, index: usize, last: usize) -> Result<(u64, &str), PluginError> {
    let (written, outcome) = entry.split_once(':').ok_or_else(|| {
        PluginError::Protocol(format!(
            "session connection {index} returned unexpected text"
        ))
    })?;
    let written = parse_count(Some(written), "session write")?;
    if index == last && outcome == "none" {
        return Err(PluginError::Protocol(
            "session cap was not observed on the final connection".to_string(),
        ));
    }
    Ok((written, outcome))
}
