use std::fmt;

use crate::capability::Capability;
use crate::protocol::RpcError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    Spawn(String),
    Timeout,
    WorkerStopped,
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
            Self::WorkerStopped => write!(formatter, "plugin worker stopped"),
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

impl PluginError {
    /// A transport failure means the session can no longer be trusted. RPC errors
    /// are normal replies from a healthy guest and must not tear the session down.
    pub fn is_transport_failure(&self) -> bool {
        matches!(
            self,
            Self::Spawn(_)
                | Self::Timeout
                | Self::WorkerStopped
                | Self::Trap(_)
                | Self::Protocol(_)
                | Self::Initialize(_)
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_failures_require_a_new_session() {
        for error in [
            PluginError::Spawn(String::new()),
            PluginError::Timeout,
            PluginError::WorkerStopped,
            PluginError::Trap(String::new()),
            PluginError::Protocol(String::new()),
            PluginError::Initialize(String::new()),
        ] {
            assert!(error.is_transport_failure(), "{error} should be transport");
        }
    }

    #[test]
    fn rpc_errors_leave_the_session_running() {
        let error = PluginError::Rpc(RpcError::new(crate::protocol::STALE_REVISION, "stale"));
        assert!(!error.is_transport_failure());
    }
}
