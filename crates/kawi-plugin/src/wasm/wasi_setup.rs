use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::FsPerms;

use crate::capability::{Capability, CapabilityRegistry, Scope};
use crate::environment::{expand_path, HostEnvironment};
use crate::error::PluginError;
use crate::manifest::LoadedPlugin;

use super::net::NetworkPolicy;
use super::WasiConfig;

pub(super) struct WasiSetup<'a> {
    pub(super) plugin: &'a LoadedPlugin,
    pub(super) capabilities: &'a CapabilityRegistry,
    pub(super) environment: &'a HostEnvironment,
    pub(super) network: NetworkPolicy,
    pub(super) stdout: MemoryOutputPipe,
    pub(super) stderr: MemoryOutputPipe,
}

#[derive(Clone, Copy)]
enum PreopenKind {
    Read,
    Write,
}

impl PreopenKind {
    fn capability(self) -> Capability {
        match self {
            Self::Read => Capability::FsRead,
            Self::Write => Capability::FsWrite,
        }
    }

    fn perms(self) -> FsPerms {
        match self {
            Self::Read => FsPerms::ReadOnly,
            Self::Write => FsPerms::ReadWrite,
        }
    }
}

pub(super) fn build_wasi(
    setup: WasiSetup<'_>,
    warnings: &mut Vec<String>,
) -> Result<wasmtime_wasi::WasiCtx, PluginError> {
    let mut config = WasiConfig::new()
        .stdout(setup.stdout.clone())
        .stderr(setup.stderr.clone())
        .network(setup.network.clone());
    config = add_preopens(config, &setup, PreopenKind::Read, warnings)?;
    config = add_preopens(config, &setup, PreopenKind::Write, warnings)?;
    Ok(config.build())
}

fn add_preopens(
    mut config: WasiConfig,
    setup: &WasiSetup<'_>,
    kind: PreopenKind,
    warnings: &mut Vec<String>,
) -> Result<WasiConfig, PluginError> {
    let capability = kind.capability();
    let Some(Scope::Paths(paths)) = setup
        .capabilities
        .scope(&setup.plugin.manifest.name, capability)
    else {
        return Ok(config);
    };
    for raw in paths {
        let Some(path) = expand_path(&raw, &setup.plugin.dir, setup.environment) else {
            warnings.push(format!(
                "skipped granted {} path {raw}: variable is unset",
                capability.name()
            ));
            continue;
        };
        let Ok(path) = std::fs::canonicalize(&path) else {
            warnings.push(format!(
                "skipped granted {} path {raw}: path does not exist",
                capability.name()
            ));
            continue;
        };
        if !path.is_dir() {
            warnings.push(format!(
                "skipped granted {} path {raw}: WASI preopens require a directory",
                capability.name()
            ));
            continue;
        }
        let guest = path.to_string_lossy();
        config = config.preopen(&path, &guest, kind.perms())?;
    }
    Ok(config)
}
