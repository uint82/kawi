use crate::error::PluginError;
use crate::protocol::{InitializeResult, HOST_METHODS, PROTOCOL_VERSION};

pub(super) fn spawn_error_message(
    plugin: &str,
    status: kawi_process::SandboxStatus,
    error: &std::io::Error,
) -> String {
    match status {
        kawi_process::SandboxStatus::Enforced => {
            format!("{plugin} cannot start under the enforced landlock ruleset: {error}")
        }
        kawi_process::SandboxStatus::Partial => {
            format!("{plugin} cannot start under a partial landlock ruleset: {error}")
        }
        kawi_process::SandboxStatus::Unavailable => format!("{plugin}: {error}"),
    }
}

pub(super) fn validate_initialize(result: &InitializeResult) -> Result<(), PluginError> {
    if result.protocol_version != PROTOCOL_VERSION {
        return Err(PluginError::Initialize(format!(
            "unsupported protocol {}, expected {}",
            result.protocol_version, PROTOCOL_VERSION
        )));
    }
    if !result
        .plugin_capabilities
        .iter()
        .any(|capability| capability == "invoke_command")
    {
        return Err(PluginError::Initialize(
            "plugin does not declare invoke_command".to_string(),
        ));
    }
    if let Some(method) = result
        .host_capabilities
        .iter()
        .find(|method| !HOST_METHODS.contains(&method.as_str()))
    {
        return Err(PluginError::Initialize(format!(
            "plugin declared unknown host capability: {method}"
        )));
    }
    Ok(())
}
