use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SandboxPolicy {
    pub read_only: Vec<PathBuf>,
    pub read_write: Vec<PathBuf>,
    pub execute: Vec<PathBuf>,
}

impl SandboxPolicy {
    pub fn is_empty(&self) -> bool {
        self.read_only.is_empty() && self.read_write.is_empty() && self.execute.is_empty()
    }
}

pub fn sandbox_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        crate::landlock::supported()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

pub fn configure_sandbox(command: &mut Command, policy: &SandboxPolicy) -> bool {
    #[cfg(target_os = "linux")]
    {
        crate::landlock::configure(command, policy)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (command, policy);
        false
    }
}
