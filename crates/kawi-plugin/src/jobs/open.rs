use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::capability::{resolve_command, CapabilityRegistry};
use crate::environment::HostEnvironment;
use crate::manifest::{LoadedPlugin, PluginRuntime};
use crate::protocol::{ProcessOpenParams, RpcError, StdinMode};

use super::events::JobEventQueue;
use super::monitor::JobStatus;
use super::output::OutputBudget;
use super::stdin::{spawn_stdin, StdinHandle};
use super::validate;
use super::JobHandle;

pub(super) fn spawn(
    plugin: &LoadedPlugin,
    capabilities: &CapabilityRegistry,
    environment: &HostEnvironment,
    params: ProcessOpenParams,
) -> Result<
    (
        JobHandle,
        std::process::ChildStdout,
        std::process::ChildStderr,
    ),
    RpcError,
> {
    let cwd = validate::validate(plugin, environment, &params)?;
    let executable = crate::capability::authorize_command(
        &plugin.manifest.name,
        capabilities,
        &params.command,
        resolve_command,
        |command| RpcError::invalid_params(format!("command not found: {command}")),
    )?;
    let mut command = Command::new(executable);
    command
        .args(&params.args)
        .current_dir(cwd)
        .stdin(stdin(&params))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in &params.env {
        command.env(key, value);
    }
    kawi_process::configure(&mut command);
    let plan = crate::process::sandbox_policy(plugin, capabilities, environment);
    kawi_process::configure_sandbox(&mut command, &plan.policy);
    let mut child = command
        .spawn()
        .map_err(|error| RpcError::invalid_params(format!("cannot start job: {error}")))?;
    let Some(stdout) = child.stdout.take() else {
        kawi_process::terminate(&mut child);
        return Err(RpcError::invalid_params("job has no stdout"));
    };
    let Some(stderr) = child.stderr.take() else {
        kawi_process::terminate(&mut child);
        return Err(RpcError::invalid_params("job has no stderr"));
    };
    let stdin = match (params.stdin, child.stdin.take()) {
        (StdinMode::Pipe, Some(stdin)) => spawn_stdin(stdin),
        (StdinMode::Null, _) => StdinHandle::disabled(),
        (StdinMode::Pipe, None) => {
            kawi_process::terminate(&mut child);
            return Err(RpcError::invalid_params("job has no stdin"));
        }
    };
    let handle = JobHandle {
        plugin: plugin.manifest.name.clone(),
        child: Arc::new(Mutex::new(child)),
        stdin,
        status: Arc::new(Mutex::new(JobStatus::new())),
        output: Arc::new(OutputBudget::new()),
        events: Arc::new(Mutex::new(JobEventQueue::new())),
        queue_events: plugin.manifest.runtime == PluginRuntime::Wasm,
    };
    Ok((handle, stdout, stderr))
}

fn stdin(params: &ProcessOpenParams) -> Stdio {
    match params.stdin {
        StdinMode::Pipe => Stdio::piped(),
        StdinMode::Null => Stdio::null(),
    }
}
