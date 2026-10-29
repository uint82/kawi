use kawi_plugin::{PluginError, RpcError, CAPABILITY_DENIED, PLUGIN_UNAVAILABLE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Outcome {
    Success,
    Denied(String),
    Crashed,
    Timeout,
    Protocol,
    QueueFull,
}

impl Outcome {
    pub(super) fn denied(capability: &str) -> Self {
        Self::Denied(capability.to_string())
    }
}

pub(super) fn normalize_result(result: &Result<serde_json::Value, PluginError>) -> Outcome {
    match result {
        Ok(_) => Outcome::Success,
        Err(error) => normalize_error(error),
    }
}

pub(super) fn normalize_error(error: &PluginError) -> Outcome {
    match error {
        PluginError::Timeout => Outcome::Timeout,
        PluginError::QueueFull => Outcome::QueueFull,
        PluginError::Protocol(_) => Outcome::Protocol,
        PluginError::Rpc(error) => normalize_rpc(error),
        PluginError::Spawn(_)
        | PluginError::ProcessExited
        | PluginError::Trap(_)
        | PluginError::Initialize(_)
        | PluginError::MissingCapabilities(_)
        | PluginError::Backoff { .. }
        | PluginError::Disabled
        | PluginError::Restart
        | PluginError::Shutdown => Outcome::Crashed,
    }
}

pub(super) fn normalize_rpc(error: &RpcError) -> Outcome {
    if error.code == PLUGIN_UNAVAILABLE {
        return Outcome::Crashed;
    }
    if error.code != CAPABILITY_DENIED {
        return Outcome::Protocol;
    }
    let capability = error
        .data
        .as_ref()
        .and_then(|data| data.get("capability"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");
    Outcome::denied(capability)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn successful_results_normalize_to_success() {
        assert_eq!(normalize_result(&Ok(json!(null))), Outcome::Success);
    }

    #[test]
    fn capability_denials_keep_the_capability_name() {
        let error = RpcError::capability_denied("editor.read", None);
        assert_eq!(
            normalize_error(&PluginError::Rpc(error)),
            Outcome::denied("editor.read")
        );
    }

    #[test]
    fn capability_denials_without_data_use_an_unknown_capability() {
        let error = RpcError::new(CAPABILITY_DENIED, "denied");
        assert_eq!(normalize_rpc(&error), Outcome::denied("unknown"));
    }

    #[test]
    fn other_rpc_errors_normalize_to_protocol() {
        assert_eq!(
            normalize_rpc(&RpcError::stale_revision(1, 2)),
            Outcome::Protocol
        );
    }

    #[test]
    fn unavailable_rpc_errors_normalize_to_crashed() {
        assert_eq!(
            normalize_rpc(&RpcError::unavailable("restarting")),
            Outcome::Crashed
        );
    }

    #[test]
    fn transport_failures_normalize_to_crashed() {
        for error in [
            PluginError::ProcessExited,
            PluginError::Trap("bad".to_string()),
            PluginError::Spawn("missing".to_string()),
            PluginError::Initialize("bad".to_string()),
            PluginError::Backoff { remaining_ms: 1 },
            PluginError::Disabled,
            PluginError::Restart,
            PluginError::Shutdown,
        ] {
            assert_eq!(normalize_error(&error), Outcome::Crashed, "{error}");
        }
    }

    #[test]
    fn timeout_queue_and_protocol_failures_keep_their_kind() {
        assert_eq!(normalize_error(&PluginError::Timeout), Outcome::Timeout);
        assert_eq!(normalize_error(&PluginError::QueueFull), Outcome::QueueFull);
        assert_eq!(
            normalize_error(&PluginError::Protocol("bad".to_string())),
            Outcome::Protocol
        );
    }

    #[test]
    fn missing_capabilities_normalize_to_crashed() {
        assert_eq!(
            normalize_error(&PluginError::MissingCapabilities(vec![
                kawi_plugin::Capability::EditorRead
            ])),
            Outcome::Crashed
        );
    }
}
