use super::super::*;
use crate::capability::parse_capability_requests;

pub(super) fn requests(text: &str) -> CapabilityRequests {
    let value: toml::Value = toml::from_str(text).expect("toml");
    parse_capability_requests(value.as_table().expect("table")).0
}

pub(super) fn canonical_command(command: &str) -> String {
    let path = kawi_process::find_executable(command).expect("command on PATH");
    std::fs::canonicalize(path)
        .expect("canonical command")
        .to_string_lossy()
        .into_owned()
}

pub(super) fn raw_effective_for(
    request_text: &str,
    permissions_text: &str,
) -> EffectiveCapabilities {
    let requests = requests(request_text);
    let file = parse_permissions(permissions_text).expect("permissions");
    effective(&requests, file.plugins.get("plugin"))
}

pub(super) fn effective_for(request_text: &str, permissions_text: &str) -> EffectiveCapabilities {
    let requests = requests(request_text);
    let text = with_approved(permissions_text, request_text);
    let file = parse_permissions(&text).expect("permissions");
    effective(&requests, file.plugins.get("plugin"))
}

pub(super) fn with_approved(permissions: &str, request: &str) -> String {
    if permissions.contains("approved") || permissions.contains("request_hash") {
        return permissions.to_string();
    }
    let approved = format!("[plugins.plugin.approved]\n{request}\n");
    if permissions.contains("[plugins.plugin]") {
        return permissions.replacen(
            "[plugins.plugin]",
            &format!("[plugins.plugin]\n\n{approved}"),
            1,
        );
    }
    if permissions.contains("[plugins.plugin.") {
        return format!("{approved}\n{permissions}");
    }
    permissions.to_string()
}
