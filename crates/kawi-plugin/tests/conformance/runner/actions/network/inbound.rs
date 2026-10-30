use std::io::{self, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

use serde_json::{json, Value};

use kawi_plugin::PluginError;

use super::super::super::super::harness::ConformanceHost;
use super::super::super::super::scenario::NetworkTarget;
use super::{accept, parse_counted, CONNECTION_BYTES};

const SEND_LIMIT: usize = 9 * 1024 * 1024;
const SEND_CHUNK: usize = 256 * 1024;
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) fn run(host: &ConformanceHost, target: NetworkTarget) -> Result<Value, PluginError> {
    let request = host.start_invocation("read-cap", json!([host.network_target(target)]))?;
    let mut stream = accept(host)?;
    stream
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .map_err(|error| PluginError::Protocol(format!("set write timeout: {error}")))?;
    let sent = send_bytes(&mut stream, SEND_LIMIT)?;
    let _ = stream.shutdown(Shutdown::Write);
    if sent < CONNECTION_BYTES as usize {
        return Err(PluginError::Protocol(format!(
            "sent {sent} bytes, expected at least {CONNECTION_BYTES}"
        )));
    }
    let result = host.wait_for_finished(request)?;
    assert_inbound_cap(&result)?;
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

fn send_bytes(stream: &mut TcpStream, limit: usize) -> Result<usize, PluginError> {
    let chunk = [0_u8; SEND_CHUNK];
    let mut sent = 0;
    while sent < limit {
        let remaining = (limit - sent).min(SEND_CHUNK);
        match stream.write(&chunk[..remaining]) {
            Ok(0) => break,
            Ok(count) => sent += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if is_terminal_send_error(&error) => break,
            Err(error) => {
                return Err(PluginError::Protocol(format!(
                    "send inbound bytes: {error}"
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
