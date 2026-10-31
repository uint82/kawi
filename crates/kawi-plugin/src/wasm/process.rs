use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::capability::{resolve_command, Capability, CapabilityRegistry, Scope};
use crate::environment::HostEnvironment;
use crate::manifest::LoadedPlugin;
use crate::protocol::{RpcError, PLUGIN_UNAVAILABLE};

use super::bindings::kawi::plugin::types::ProcessOutput;
use super::convert::process_output;

const MIN_TIMEOUT_MS: u32 = 100;
const MAX_TIMEOUT_MS: u32 = 300_000;
const MAX_OUTPUT_BYTES: usize = 64 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);

pub struct ProcessRunner {
    plugin: LoadedPlugin,
    capabilities: CapabilityRegistry,
    environment: HostEnvironment,
}

impl ProcessRunner {
    pub fn new(
        plugin: LoadedPlugin,
        capabilities: CapabilityRegistry,
        environment: HostEnvironment,
    ) -> Self {
        Self {
            plugin,
            capabilities,
            environment,
        }
    }

    pub fn run(
        &self,
        command: &str,
        args: &[String],
        timeout_ms: u32,
    ) -> Result<ProcessOutput, RpcError> {
        let path = self.authorize(command)?;
        let timeout =
            Duration::from_millis(u64::from(timeout_ms.clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS)));
        let mut command = Command::new(path);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        kawi_process::configure(&mut command);
        let policy = self.sandbox();
        kawi_process::configure_sandbox(&mut command, &policy);
        let mut child = command
            .spawn()
            .map_err(|error| RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))?;
        let stdout = read_stream(child.stdout.take());
        let stderr = read_stream(child.stderr.take());
        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
                Ok(None) => {
                    kawi_process::terminate(&mut child);
                    return Err(RpcError::new(PLUGIN_UNAVAILABLE, "process timed out"));
                }
                Err(error) => {
                    kawi_process::terminate(&mut child);
                    return Err(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()));
                }
            }
        };
        let stdout = join_stream(stdout);
        let stderr = join_stream(stderr);
        Ok(process_output(stdout, stderr, status.code()))
    }

    fn authorize(&self, command: &str) -> Result<std::path::PathBuf, RpcError> {
        authorize_command(
            &self.plugin.manifest.name,
            &self.capabilities,
            command,
            resolve_command,
        )
    }

    pub fn sandbox(&self) -> kawi_process::SandboxPolicy {
        crate::process::sandbox_policy(&self.plugin, &self.capabilities, &self.environment).policy
    }
}

pub(crate) fn authorize_command(
    plugin: &str,
    capabilities: &CapabilityRegistry,
    command: &str,
    resolve: impl Fn(&str) -> Option<std::path::PathBuf>,
) -> Result<std::path::PathBuf, RpcError> {
    if !capabilities.has(plugin, Capability::ProcessSpawn) {
        return Err(RpcError::capability_denied(
            Capability::ProcessSpawn.name(),
            None,
        ));
    }
    let requested = resolve(command).ok_or_else(|| command_not_found(command))?;
    let Some(Scope::Commands(commands)) = capabilities.scope(plugin, Capability::ProcessSpawn)
    else {
        return Err(capability_denied(command));
    };
    if commands
        .iter()
        .any(|granted| resolve(granted).is_some_and(|granted| granted == requested))
    {
        Ok(requested)
    } else {
        Err(capability_denied(command))
    }
}

fn command_not_found(command: &str) -> RpcError {
    RpcError::new(PLUGIN_UNAVAILABLE, format!("command not found: {command}"))
}

fn capability_denied(command: &str) -> RpcError {
    RpcError::capability_denied(
        Capability::ProcessSpawn.name(),
        Some(serde_json::json!({ "command": command })),
    )
}

fn read_stream(stream: Option<impl Read + Send + 'static>) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let Some(mut stream) = stream else {
            return String::new();
        };
        let mut buffer = [0_u8; 8192];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    let remaining = MAX_OUTPUT_BYTES.saturating_sub(bytes.len());
                    bytes.extend_from_slice(&buffer[..read.min(remaining)]);
                }
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

fn join_stream(stream: thread::JoinHandle<String>) -> String {
    stream.join().unwrap_or_default()
}

#[cfg(test)]
mod tests;
