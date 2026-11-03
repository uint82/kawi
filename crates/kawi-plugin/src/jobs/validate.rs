use std::path::{Path, PathBuf};

use crate::environment::{expand_path, HostEnvironment};
use crate::manifest::LoadedPlugin;
use crate::protocol::{
    ProcessOpenParams, RpcError, MAX_JOB_ARGUMENTS, MAX_JOB_ARGUMENT_BYTES,
    MAX_JOB_ARGUMENT_TOTAL_BYTES, MAX_JOB_ENV_BYTES, MAX_JOB_ENV_ENTRIES, MAX_JOB_ENV_KEY_BYTES,
    MAX_JOB_ENV_VALUE_BYTES,
};

pub(super) fn validate(
    plugin: &LoadedPlugin,
    environment: &HostEnvironment,
    params: &ProcessOpenParams,
) -> Result<PathBuf, RpcError> {
    validate_command(params)?;
    validate_args(params)?;
    validate_env(params)?;
    validate_cwd(plugin, environment, params.cwd.as_deref())
}

fn validate_command(params: &ProcessOpenParams) -> Result<(), RpcError> {
    if params.command.trim().is_empty() {
        return Err(RpcError::invalid_params("job command must not be empty"));
    }
    Ok(())
}

fn validate_args(params: &ProcessOpenParams) -> Result<(), RpcError> {
    if params.args.len() > MAX_JOB_ARGUMENTS {
        return Err(RpcError::invalid_params("too many job arguments"));
    }
    if params
        .args
        .iter()
        .any(|arg| arg.len() > MAX_JOB_ARGUMENT_BYTES)
    {
        return Err(RpcError::invalid_params("job argument is too large"));
    }
    let total = params
        .args
        .iter()
        .fold(0_usize, |total, arg| total.saturating_add(arg.len()));
    if total > MAX_JOB_ARGUMENT_TOTAL_BYTES {
        return Err(RpcError::invalid_params(
            "job arguments exceed the maximum size",
        ));
    }
    Ok(())
}

fn validate_env(params: &ProcessOpenParams) -> Result<(), RpcError> {
    if params.env.len() > MAX_JOB_ENV_ENTRIES {
        return Err(RpcError::invalid_params("too many job environment entries"));
    }
    let mut total = 0_usize;
    for (key, value) in &params.env {
        validate_env_entry(key, value)?;
        total = total.saturating_add(key.len()).saturating_add(value.len());
    }
    if total > MAX_JOB_ENV_BYTES {
        return Err(RpcError::invalid_params(
            "job environment exceeds the maximum size",
        ));
    }
    Ok(())
}

fn validate_env_entry(key: &str, value: &str) -> Result<(), RpcError> {
    if key.is_empty() || key.contains('=') || key.contains('\0') {
        return Err(RpcError::invalid_params("job environment key is invalid"));
    }
    if key.len() > MAX_JOB_ENV_KEY_BYTES {
        return Err(RpcError::invalid_params("job environment key is too large"));
    }
    if value.len() > MAX_JOB_ENV_VALUE_BYTES || value.contains('\0') {
        return Err(RpcError::invalid_params("job environment value is invalid"));
    }
    Ok(())
}

fn validate_cwd(
    plugin: &LoadedPlugin,
    environment: &HostEnvironment,
    cwd: Option<&str>,
) -> Result<PathBuf, RpcError> {
    let Some(raw) = cwd else {
        return Ok(plugin.dir.clone());
    };
    let Some(expanded) = expand_path(raw, &plugin.dir, environment) else {
        return Err(RpcError::invalid_params("job cwd has an unknown variable"));
    };
    let candidate = if expanded.is_absolute() {
        expanded
    } else {
        plugin.dir.join(expanded)
    };
    let path = std::fs::canonicalize(&candidate)
        .map_err(|error| RpcError::invalid_params(format!("job cwd is invalid: {error}")))?;
    if !path.is_dir() {
        return Err(RpcError::invalid_params("job cwd is not a directory"));
    }
    let roots = [
        canonical(&plugin.dir),
        environment.workspace.as_deref().and_then(canonical),
        environment.config.as_deref().and_then(canonical),
    ];
    if roots
        .iter()
        .flatten()
        .any(|root| path == *root || path.starts_with(root))
    {
        return Ok(path);
    }
    Err(RpcError::invalid_params(
        "job cwd is outside the allowed roots",
    ))
}

fn canonical(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}
