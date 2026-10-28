use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostEnvironment {
    pub workspace: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub home: Option<PathBuf>,
}

impl HostEnvironment {
    pub fn from_paths(workspace: Option<PathBuf>, config: Option<PathBuf>) -> Self {
        Self {
            workspace,
            config,
            home: std::env::var_os("HOME").map(PathBuf::from),
        }
    }
}
