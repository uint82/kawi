use std::fmt;

use crate::capability::Capability;
use crate::protocol::RpcError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    Spawn(String),
    Timeout,
    ProcessExited,
    Trap(String),
    QueueFull,
    Protocol(String),
    Initialize(String),
    MissingCapabilities(Vec<Capability>),
    Backoff { remaining_ms: u64 },
    Disabled,
    Rpc(RpcError),
    Restart,
    Shutdown,
}

impl fmt::Display for PluginError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(message) => write!(formatter, "cannot start plugin: {message}"),
            Self::Timeout => write!(formatter, "plugin request timed out"),
            Self::ProcessExited => write!(formatter, "plugin process exited"),
            Self::Trap(message) => write!(formatter, "plugin trapped: {message}"),
            Self::QueueFull => write!(formatter, "plugin queue is full"),
            Self::Protocol(message) => write!(formatter, "plugin protocol error: {message}"),
            Self::Initialize(message) => {
                write!(formatter, "plugin initialization failed: {message}")
            }
            Self::MissingCapabilities(capabilities) => {
                write_missing_capabilities(formatter, capabilities)
            }
            Self::Backoff { remaining_ms } => {
                write!(
                    formatter,
                    "plugin is restarting, retry in {remaining_ms} ms"
                )
            }
            Self::Disabled => write!(
                formatter,
                "plugin is disabled after repeated crashes, use :plugin restart"
            ),
            Self::Rpc(error) => write!(formatter, "plugin error {}: {}", error.code, error.message),
            Self::Restart => write!(formatter, "plugin restarted; invocation canceled"),
            Self::Shutdown => write!(formatter, "plugin host is shutting down"),
        }
    }
}

impl std::error::Error for PluginError {}

fn write_missing_capabilities(
    formatter: &mut fmt::Formatter<'_>,
    capabilities: &[Capability],
) -> fmt::Result {
    formatter.write_str("plugin requests ")?;
    let mut capabilities = capabilities.iter();
    if let Some(capability) = capabilities.next() {
        formatter.write_str(capability.name())?;
    }
    for capability in capabilities {
        write!(formatter, ", {}", capability.name())?;
    }
    formatter.write_str(", not granted")
}
