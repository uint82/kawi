use super::*;
use crate::capability::parse_capability_requests;

fn requests(text: &str) -> CapabilityRequests {
    let value: toml::Value = toml::from_str(text).expect("toml");
    parse_capability_requests(value.as_table().expect("table")).0
}

fn effective_for(request_text: &str, permissions_text: &str) -> EffectiveCapabilities {
    let requests = requests(request_text);
    let file = parse_permissions(permissions_text).expect("permissions");
    effective(&requests, file.plugins.get("plugin"))
}

#[test]
fn default_tier_is_granted_and_can_be_revoked() {
    let granted = effective_for(r#""ui.message" = true"#, "[plugins.plugin]\n");
    assert!(granted.has(Capability::UiMessage));

    let revoked = effective_for(
        r#""ui.message" = true"#,
        r#"
[plugins.plugin.grant]
"ui.message" = false
"#,
    );
    assert!(!revoked.has(Capability::UiMessage));
    assert_eq!(revoked.missing, vec![Capability::UiMessage]);
}

#[test]
fn path_grants_narrow_requests() {
    let granted = effective_for(
        r#""fs.read" = { paths = ["$WORKSPACE"] }"#,
        r#"
[plugins.plugin.grant]
"fs.read" = { paths = ["$WORKSPACE/src"] }
"#,
    );
    assert_eq!(
        granted.scope(Capability::FsRead),
        Some(&Scope::Paths(vec!["$WORKSPACE/src".to_string()]))
    );
}

#[test]
fn broader_path_grants_do_not_cover_requests() {
    let granted = effective_for(
        r#""fs.read" = { paths = ["$WORKSPACE/src"] }"#,
        r#"
[plugins.plugin.grant]
"fs.read" = { paths = ["$WORKSPACE"] }
"#,
    );
    assert!(!granted.has(Capability::FsRead));
    assert_eq!(granted.missing, vec![Capability::FsRead]);
}

#[test]
fn command_and_host_grants_are_exact() {
    let granted = effective_for(
        r#"
"process.spawn" = { commands = ["git", "rg"] }
"net.connect" = { hosts = ["api.example.com:443", "other.example.com:443"] }
"#,
        r#"
[plugins.plugin.grant]
"process.spawn" = { commands = ["git"] }
"net.connect" = { hosts = ["API.EXAMPLE.COM:443"] }
"#,
    );
    assert_eq!(
        granted.scope(Capability::ProcessSpawn),
        Some(&Scope::Commands(vec!["git".to_string()]))
    );
    assert_eq!(
        granted.scope(Capability::NetConnect),
        Some(&Scope::Hosts(vec!["api.example.com:443".to_string()]))
    );
}

#[test]
fn unrequested_grants_are_stale() {
    let granted = effective_for(
        r#""editor.read" = true"#,
        r#"
[plugins.plugin.grant]
"editor.read" = true
"fs.read" = { paths = ["$HOME"] }
"#,
    );
    assert!(granted.has(Capability::EditorRead));
    assert_eq!(granted.stale, vec![Capability::FsRead]);
}

#[test]
fn invalid_entries_are_ignored_and_malformed_files_fail_parse() {
    let granted = effective_for(
        r#""editor.read" = true"#,
        r#"
[plugins.plugin.grant]
"editor.read" = true
"fs.read" = { paths = ["relative"] }
"#,
    );
    assert!(granted.has(Capability::EditorRead));
    assert_eq!(granted.warnings.len(), 1);
    assert!(parse_permissions("[plugins").is_err());
}
