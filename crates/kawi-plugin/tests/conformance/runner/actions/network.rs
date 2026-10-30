use std::io::Read;
use std::time::Duration;

use serde_json::{json, Value};

use kawi_plugin::{PluginError, PluginStatus};

use super::super::super::harness::{ConformanceHost, Runtime, PLUGIN_NAME};
use super::super::super::scenario::{network_case, NetworkTarget, Scenario};

const EXPECTED_OUTBOUND_BYTES: u64 = 8 * 1024 * 1024;

pub(crate) fn run_network_probe(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    assert_eq!(runtime, Runtime::Wasm);
    let case = network_case(scenario.id);
    match case.command {
        "hold" => run_restart_close(host),
        "write-cap" => run_outbound_cap(host, case.target, "write-cap"),
        "write-vectored" => run_outbound_cap(host, case.target, "write-vectored"),
        "write-strict" => run_outbound_cap_no_reader(host, case.target),
        "write-straddle" => run_outbound_cap_straddle(host, case.target),
        _ => host.invoke(case.command, json!([host.network_target(case.target)])),
    }
}

fn run_outbound_cap(
    host: &ConformanceHost,
    target: NetworkTarget,
    command: &str,
) -> Result<Value, PluginError> {
    let request = host.start_invocation(command, json!([host.network_target(target)]))?;
    let received = drain_allowed(host)?;
    let result = host.wait_for_finished(request)?;
    assert_outbound_cap(received, &result)?;
    assert_plugin_ready(host)?;
    Ok(result)
}

fn assert_plugin_ready(host: &ConformanceHost) -> Result<(), PluginError> {
    let status = host.host.status(PLUGIN_NAME);
    if status == Some(PluginStatus::Ready) {
        return Ok(());
    }
    Err(PluginError::Protocol(format!(
        "cap hit changed plugin status to {status:?}"
    )))
}

fn run_outbound_cap_no_reader(
    host: &ConformanceHost,
    target: NetworkTarget,
) -> Result<Value, PluginError> {
    let request = host.start_invocation("write-strict", json!([host.network_target(target)]))?;
    let _stream = host
        .accept_allowed()
        .map_err(|error| PluginError::Protocol(format!("accept capped socket: {error}")))?;
    host.wait_for_finished(request)
}

fn run_outbound_cap_straddle(
    host: &ConformanceHost,
    target: NetworkTarget,
) -> Result<Value, PluginError> {
    let request = host.start_invocation("write-straddle", json!([host.network_target(target)]))?;
    let received = drain_allowed(host)?;
    let result = host.wait_for_finished(request)?;
    assert_received(received)?;
    let (prefix, straddle, outcome) = parse_straddle(&result)?;
    assert_straddle(prefix, straddle, outcome)?;
    Ok(result)
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

fn parse_count(value: Option<&str>, label: &str) -> Result<u64, PluginError> {
    value
        .ok_or_else(|| PluginError::Protocol(format!("missing {label} count")))?
        .parse::<u64>()
        .map_err(|error| PluginError::Protocol(format!("invalid {label} count: {error}")))
}

fn assert_straddle(prefix: u64, straddle: u64, outcome: &str) -> Result<(), PluginError> {
    if prefix != EXPECTED_OUTBOUND_BYTES - 1 {
        return Err(PluginError::Protocol(format!(
            "straddle prefix wrote {prefix} bytes, expected {}",
            EXPECTED_OUTBOUND_BYTES - 1
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

fn drain_allowed(host: &ConformanceHost) -> Result<u64, PluginError> {
    let mut stream = host
        .accept_allowed()
        .map_err(|error| PluginError::Protocol(format!("accept capped socket: {error}")))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| PluginError::Protocol(format!("set read timeout: {error}")))?;
    let mut received = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => return Ok(received),
            Ok(count) => received += count as u64,
            Err(error) => {
                return Err(PluginError::Protocol(format!(
                    "read capped socket: {error}"
                )))
            }
        }
    }
}

fn assert_outbound_cap(received: u64, result: &Value) -> Result<(), PluginError> {
    assert_received(received)?;
    let (written, outcome) = parse_written(result)?;
    assert_written(written, outcome)?;
    assert_cap_observed(outcome)
}

fn assert_received(received: u64) -> Result<(), PluginError> {
    if received == EXPECTED_OUTBOUND_BYTES {
        return Ok(());
    }
    Err(PluginError::Protocol(format!(
        "socket received {received} bytes, expected {EXPECTED_OUTBOUND_BYTES}"
    )))
}

fn parse_written(result: &Value) -> Result<(u64, &str), PluginError> {
    let text = result
        .as_str()
        .ok_or_else(|| PluginError::Protocol("capped write returned no text".to_string()))?;
    let (written, outcome) = text.split_once(':').ok_or_else(|| {
        PluginError::Protocol(format!("capped write returned unexpected text: {text}"))
    })?;
    let written = written.parse::<u64>().map_err(|error| {
        PluginError::Protocol(format!("capped write returned invalid count: {error}"))
    })?;
    Ok((written, outcome))
}

fn assert_written(written: u64, outcome: &str) -> Result<(), PluginError> {
    if written == EXPECTED_OUTBOUND_BYTES {
        return Ok(());
    }
    Err(PluginError::Protocol(format!(
        "guest wrote {written} bytes, expected {EXPECTED_OUTBOUND_BYTES}: {outcome}"
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
