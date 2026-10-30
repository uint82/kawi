pub(super) use kawi_plugin::{normalize_error, normalize_result, normalize_rpc, Outcome};

#[cfg(test)]
mod tests {
    use kawi_plugin::{PluginError, RpcError, CAPABILITY_DENIED};
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
