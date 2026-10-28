use std::fmt;

use crate::protocol::RpcError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    Spawn(String),
    Timeout,
    ProcessExited,
    QueueFull,
    Protocol(String),
    Initialize(String),
    Backoff { remaining_ms: u64 },
    Disabled,
    Rpc(RpcError),
    Shutdown,
}

impl fmt::Display for PluginError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(message) => write!(formatter, "cannot start plugin: {message}"),
            Self::Timeout => write!(formatter, "plugin request timed out"),
            Self::ProcessExited => write!(formatter, "plugin process exited"),
            Self::QueueFull => write!(formatter, "plugin event queue is full"),
            Self::Protocol(message) => write!(formatter, "plugin protocol error: {message}"),
            Self::Initialize(message) => {
                write!(formatter, "plugin initialization failed: {message}")
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
            Self::Shutdown => write!(formatter, "plugin host is shutting down"),
        }
    }
}

impl std::error::Error for PluginError {}
