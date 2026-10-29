use std::path::Path;

use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxBuilder};

use crate::error::PluginError;

pub struct WasiConfig {
    builder: WasiCtxBuilder,
}

impl WasiConfig {
    pub fn new() -> Self {
        let mut builder = WasiCtxBuilder::new();
        builder.args(&[] as &[&str]);
        builder.envs(&[] as &[(&str, &str)]);
        builder.allow_ip_name_lookup(false);
        builder.allow_tcp(false);
        builder.allow_udp(false);
        Self { builder }
    }

    pub fn preopen(
        mut self,
        host_path: &Path,
        guest_path: &str,
        perms: FsPerms,
    ) -> Result<Self, PluginError> {
        self.builder
            .preopened_dir(host_path, guest_path, perms)
            .map_err(|error| PluginError::Initialize(format!("wasi preopen: {error}")))?;
        Ok(self)
    }

    pub fn build(self) -> WasiCtx {
        let mut builder = self.builder;
        builder.build()
    }
}

impl Default for WasiConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn preopens_existing_directories() {
        let dir = std::env::temp_dir().join(format!("kawi-wasi-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let config = WasiConfig::new()
            .preopen(&dir, "/data", FsPerms::ReadOnly)
            .unwrap();
        let _ = config.build();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_missing_preopens() {
        let dir = std::env::temp_dir().join(format!("kawi-wasi-missing-{}", std::process::id()));
        let error = WasiConfig::new()
            .preopen(&dir, "/data", FsPerms::ReadOnly)
            .err()
            .unwrap();
        assert!(error.to_string().contains("wasi preopen"));
    }
}
