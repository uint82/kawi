use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxStatus {
    Unavailable,
    Partial,
    Enforced,
}

impl SandboxStatus {
    pub fn is_enforced(self) -> bool {
        self == Self::Enforced
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SandboxPolicy {
    pub read_only: Vec<PathBuf>,
    pub read_write: Vec<PathBuf>,
    pub execute: Vec<PathBuf>,
}

pub fn sandbox_status() -> SandboxStatus {
    #[cfg(target_os = "linux")]
    {
        crate::landlock::status()
    }
    #[cfg(not(target_os = "linux"))]
    {
        SandboxStatus::Unavailable
    }
}

pub fn configure_sandbox(command: &mut Command, policy: &SandboxPolicy) -> SandboxStatus {
    #[cfg(target_os = "linux")]
    {
        crate::landlock::configure(command, policy)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (command, policy);
        SandboxStatus::Unavailable
    }
}
