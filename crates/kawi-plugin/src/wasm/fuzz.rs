use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use bytes::Bytes;
use wasmtime_wasi::p2::{
    InputStream as InputStreamTrait, OutputStream as OutputStreamTrait, StreamError,
};
use wasmtime_wasi::sockets::SocketAddrUse;

use super::bridge::CallDeadline;
use super::byte_budget::{SessionBudget, MAX_BYTES_PER_CONNECTION};
use super::inbound_cap::CappedInputStream;
use super::net::{is_public_ip, NetworkPolicy};
use super::outbound_cap::CappedOutputStream;
use super::process::authorize_command;
use super::test_streams::{RecordingStream, SourceStream};
use crate::capability::{Capability, CapabilityRegistry, EffectiveCapabilities, HostPort, Scope};
use crate::protocol::RpcError;

pub const CONNECTION_CAP: u64 = MAX_BYTES_PER_CONNECTION;
const FUZZ_DEADLINE: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuzzStreamResult {
    Ok(usize),
    Closed,
    Failed(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzReadSummary {
    pub consumed: usize,
    pub session_remaining: u64,
    pub results: Vec<FuzzStreamResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzWriteSummary {
    pub written: usize,
    pub session_remaining: u64,
    pub results: Vec<FuzzStreamResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuzzWriteOp {
    CheckWrite,
    Write(Vec<u8>),
    WriteZeroes(usize),
    Flush,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSummary {
    pub enabled: bool,
    pub name_lookup: bool,
    pub allowed: Vec<SocketAddr>,
    pub names: BTreeMap<String, Vec<IpAddr>>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessDecision {
    pub allowed: bool,
    pub path: Option<String>,
    pub args: Vec<String>,
    pub argv: Vec<String>,
    pub error: Option<RpcError>,
}

pub fn stream_reads(data: &[u8], session_limit: u64, sizes: &[usize]) -> FuzzReadSummary {
    let session = SessionBudget::with_limit(session_limit);
    let deadline = CallDeadline::new(FUZZ_DEADLINE);
    let source = SourceStream::new(Bytes::copy_from_slice(data));
    let mut stream = CappedInputStream::new(Box::new(source), deadline, session.clone());
    let mut consumed = 0;
    let results = sizes
        .iter()
        .map(|size| match stream.read(*size) {
            Ok(bytes) => {
                consumed += bytes.len();
                FuzzStreamResult::Ok(bytes.len())
            }
            Err(error) => stream_error(error),
        })
        .collect();
    FuzzReadSummary {
        consumed,
        session_remaining: session.remaining(),
        results,
    }
}

pub fn stream_writes(session_limit: u64, permit: usize, ops: &[FuzzWriteOp]) -> FuzzWriteSummary {
    let session = SessionBudget::with_limit(session_limit);
    let deadline = CallDeadline::new(FUZZ_DEADLINE);
    let (inner, written) = RecordingStream::new(permit);
    let mut stream = CappedOutputStream::new(Box::new(inner), deadline, session.clone());
    let results = ops
        .iter()
        .map(|op| match op {
            FuzzWriteOp::CheckWrite => stream
                .check_write()
                .map_or_else(stream_error, FuzzStreamResult::Ok),
            FuzzWriteOp::Write(bytes) => stream
                .write(Bytes::copy_from_slice(bytes))
                .map_or_else(stream_error, |_| FuzzStreamResult::Ok(bytes.len())),
            FuzzWriteOp::WriteZeroes(len) => stream
                .write_zeroes(*len)
                .map_or_else(stream_error, |_| FuzzStreamResult::Ok(*len)),
            FuzzWriteOp::Flush => stream
                .flush()
                .map_or_else(stream_error, |_| FuzzStreamResult::Ok(0)),
        })
        .collect();
    let written = written
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .len();
    FuzzWriteSummary {
        written,
        session_remaining: session.remaining(),
        results,
    }
}

pub fn host_port(value: &str) -> Result<(String, u16), String> {
    HostPort::parse(value).map(|host| (host.to_string(), host.port()))
}

pub fn host_port_ip(value: &str) -> Option<IpAddr> {
    HostPort::parse(value).ok().and_then(|host| host.ip())
}

pub fn is_public_address(ip: IpAddr) -> bool {
    is_public_ip(ip)
}

pub fn network_policy(hosts: &[String], answers: &[SocketAddr]) -> NetworkSummary {
    let policy = network_from(hosts, answers);
    NetworkSummary {
        enabled: policy.is_enabled(),
        name_lookup: policy.allows_name_lookup(),
        allowed: policy.allowed().iter().copied().collect(),
        names: policy.names().clone(),
        warnings: policy.warnings().to_vec(),
    }
}

pub fn network_allows_connect(
    hosts: &[String],
    answers: &[SocketAddr],
    address: SocketAddr,
) -> bool {
    network_from(hosts, answers).allows(address, SocketAddrUse::TcpConnect)
}

pub fn network_allows_bind(hosts: &[String], answers: &[SocketAddr], address: SocketAddr) -> bool {
    network_from(hosts, answers).allows(address, SocketAddrUse::TcpBind)
}

pub fn network_resolve_name(
    hosts: &[String],
    answers: &[SocketAddr],
    name: &str,
) -> Result<Vec<IpAddr>, String> {
    network_from(hosts, answers)
        .resolve_name(name)
        .map_err(|error| error.to_string())
}

fn network_from(hosts: &[String], answers: &[SocketAddr]) -> NetworkPolicy {
    NetworkPolicy::from_hosts_with(hosts, |_| Ok(answers.to_vec()))
}

pub fn mock_process_path(command: &str) -> Option<String> {
    if command.is_empty() {
        return None;
    }
    let mut path = String::from("/fuzz/");
    for byte in command.bytes() {
        path.push_str(&format!("{byte:02x}"));
    }
    Some(path)
}

pub fn process_authorize(
    plugin: &str,
    command: &str,
    args: &[String],
    granted: &[String],
    has_capability: bool,
) -> ProcessDecision {
    let mut effective = EffectiveCapabilities::default();
    if has_capability {
        effective
            .granted
            .insert(Capability::ProcessSpawn, Scope::Commands(granted.to_vec()));
    }
    let registry = CapabilityRegistry::default();
    registry.replace(BTreeMap::from([(plugin.to_string(), effective)]));
    match authorize_command(plugin, &registry, command, |command| {
        mock_process_path(command).map(PathBuf::from)
    }) {
        Ok(path) => {
            let path = path.to_string_lossy().into_owned();
            let mut argv = Vec::with_capacity(args.len() + 1);
            argv.push(path.clone());
            argv.extend(args.iter().cloned());
            ProcessDecision {
                allowed: true,
                path: Some(path),
                args: args.to_vec(),
                argv,
                error: None,
            }
        }
        Err(error) => ProcessDecision {
            allowed: false,
            path: None,
            args: args.to_vec(),
            argv: Vec::new(),
            error: Some(error),
        },
    }
}

fn stream_error(error: StreamError) -> FuzzStreamResult {
    match error {
        StreamError::Closed => FuzzStreamResult::Closed,
        StreamError::LastOperationFailed(_) => FuzzStreamResult::Failed("last-operation-failed"),
        StreamError::Trap(_) => FuzzStreamResult::Failed("trap"),
    }
}
