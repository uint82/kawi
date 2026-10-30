mod inbound;
mod outbound;
mod restart;
mod session;
mod splice;

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::{json, Value};

use kawi_plugin::{PluginError, PluginStatus};

use super::super::super::harness::{ConformanceHost, Runtime, PLUGIN_NAME};
use super::super::super::scenario::{network_case, Scenario};

const READ_TIMEOUT: Duration = Duration::from_secs(5);
const READ_CHUNK: usize = 256 * 1024;
pub(super) const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_CHUNK: usize = 256 * 1024;
pub(super) const CONNECTION_BYTES: u64 = 8 * 1024 * 1024;
pub(super) const SESSION_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) fn run_network_probe(
    host: &ConformanceHost,
    runtime: Runtime,
    scenario: &Scenario,
) -> Result<Value, PluginError> {
    assert_eq!(runtime, Runtime::Wasm);
    let case = network_case(scenario.id);
    match case.command {
        "hold" => restart::run(host),
        "write-cap" => outbound::run(host, case.target, "write-cap"),
        "write-vectored" => outbound::run(host, case.target, "write-vectored"),
        "write-strict" => outbound::run_no_reader(host, case.target),
        "write-straddle" => outbound::run_straddle(host, case.target),
        "write-session" => session::run(host, case.target),
        "write-close" => session::run_close(host, case.target),
        "mixed-cap" => session::run_mixed(host, case.target),
        "splice-cap" => splice::run(host, case.target),
        "read-cap" => inbound::run(host, case.target),
        "read-straddle" => inbound::run_straddle(host, case.target),
        "read-strict" => inbound::run_no_sender(host, case.target),
        _ => host.invoke(case.command, json!([host.network_target(case.target)])),
    }
}

pub(super) fn accept(host: &ConformanceHost) -> Result<TcpStream, PluginError> {
    let stream = host
        .accept_allowed()
        .map_err(|error| PluginError::Protocol(format!("accept network socket: {error}")))?;
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .map_err(|error| PluginError::Protocol(format!("set read timeout: {error}")))?;
    Ok(stream)
}

pub(super) fn drain(stream: &mut TcpStream) -> Result<u64, PluginError> {
    let mut received = 0_u64;
    let mut buffer = [0_u8; READ_CHUNK];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => return Ok(received),
            Ok(count) => received += count as u64,
            Err(error) => {
                return Err(PluginError::Protocol(format!(
                    "read network socket: {error}"
                )))
            }
        }
    }
}

pub(super) fn drain_allowed(host: &ConformanceHost) -> Result<u64, PluginError> {
    let mut stream = accept(host)?;
    drain(&mut stream)
}

pub(super) fn send_bytes(stream: &mut TcpStream, limit: usize) -> Result<usize, PluginError> {
    let chunk = [0_u8; WRITE_CHUNK];
    let mut sent = 0;
    while sent < limit {
        let remaining = (limit - sent).min(WRITE_CHUNK);
        match stream.write(&chunk[..remaining]) {
            Ok(0) => break,
            Ok(count) => sent += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if is_terminal_send_error(&error) => break,
            Err(error) => {
                return Err(PluginError::Protocol(format!(
                    "send network bytes: {error}"
                )))
            }
        }
    }
    Ok(sent)
}

fn is_terminal_send_error(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock
            | io::ErrorKind::TimedOut
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionReset
    )
}

pub(super) fn parse_count(value: Option<&str>, label: &str) -> Result<u64, PluginError> {
    value
        .ok_or_else(|| PluginError::Protocol(format!("missing {label} count")))?
        .parse::<u64>()
        .map_err(|error| PluginError::Protocol(format!("invalid {label} count: {error}")))
}

pub(super) fn parse_counted<'a>(
    result: &'a Value,
    label: &str,
) -> Result<(u64, &'a str), PluginError> {
    let text = result
        .as_str()
        .ok_or_else(|| PluginError::Protocol(format!("{label} returned no text")))?;
    let (count, outcome) = text.split_once(':').ok_or_else(|| {
        PluginError::Protocol(format!("{label} returned unexpected text: {text}"))
    })?;
    let count = parse_count(Some(count), label)?;
    Ok((count, outcome))
}

pub(super) fn assert_received(received: u64, expected: u64) -> Result<(), PluginError> {
    if received == expected {
        return Ok(());
    }
    Err(PluginError::Protocol(format!(
        "socket received {received} bytes, expected {expected}"
    )))
}

pub(super) fn assert_plugin_ready(host: &ConformanceHost) -> Result<(), PluginError> {
    let status = host.host.status(PLUGIN_NAME);
    if status == Some(PluginStatus::Ready) {
        return Ok(());
    }
    Err(PluginError::Protocol(format!(
        "cap hit changed plugin status to {status:?}"
    )))
}

pub(super) fn assert_healthy_after_cap(host: &ConformanceHost) -> Result<(), PluginError> {
    assert_plugin_ready(host)?;
    let result = host.invoke("ping", json!(null))?;
    if result.as_str() != Some("pong") {
        return Err(PluginError::Protocol(format!(
            "ping after cap returned {result}"
        )));
    }
    assert_plugin_ready(host)
}
