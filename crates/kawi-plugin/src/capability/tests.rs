use super::*;

fn requests(text: &str) -> (CapabilityRequests, Vec<String>) {
    let value: toml::Value = toml::from_str(text).expect("toml");
    parse_capability_requests(value.as_table().expect("table"))
}

#[test]
fn parses_unscoped_and_scoped_requests() {
    let (requests, warnings) = requests(
        r#"
"editor.read" = true
"editor.execute" = { commands = ["edit.format", "file.save"] }
"fs.read" = { paths = ["$WORKSPACE/src", "$WORKSPACE/src"] }
"process.spawn" = { commands = ["git"] }
"net.connect" = { hosts = ["API.EXAMPLE.COM:443"] }
"#,
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(requests.get(Capability::EditorRead), Some(&Scope::Unscoped));
    assert_eq!(
        requests.get(Capability::EditorExecute),
        Some(&Scope::EditorCommands(vec![
            "edit.format".to_string(),
            "file.save".to_string()
        ]))
    );
    assert_eq!(
        requests.get(Capability::FsRead),
        Some(&Scope::Paths(vec!["$WORKSPACE/src".to_string()]))
    );
    assert_eq!(
        requests.get(Capability::NetConnect),
        Some(&Scope::Hosts(vec!["api.example.com:443".to_string()]))
    );
}

#[test]
fn rejects_invalid_editor_commands() {
    let (requests, warnings) = requests(
        r#"
"editor.execute" = { commands = ["", "Edit.Format", "../file", "edit format"] }
"#,
    );
    assert!(!requests.contains(Capability::EditorExecute));
    assert_eq!(warnings.len(), 1);
}

#[test]
fn warns_and_ignores_invalid_requests() {
    let (requests, warnings) = requests(
        r#"
"editor.read" = true
"unknown.cap" = true
"fs.read" = true
"net.connect" = { hosts = ["*.example.com:443"] }
"#,
    );
    assert!(requests.contains(Capability::EditorRead));
    assert!(!requests.contains(Capability::FsRead));
    assert_eq!(warnings.len(), 3);
}

#[test]
fn hash_is_stable_under_reordering_and_changes_on_scope_widening() {
    let (first, _) = requests(
        r#"
"editor.read" = true
"fs.read" = { paths = ["$WORKSPACE/b", "$WORKSPACE/a"] }
"#,
    );
    let (second, _) = requests(
        r#"
"fs.read" = { paths = ["$WORKSPACE/a", "$WORKSPACE/b"] }
"editor.read" = true
"#,
    );
    let (wider, _) = requests(
        r#"
"editor.read" = true
"fs.read" = { paths = ["$WORKSPACE"] }
"#,
    );
    let (added, _) = requests(
        r#"
"editor.read" = true
"fs.read" = { paths = ["$WORKSPACE/a", "$WORKSPACE/b"] }
"ui.message" = true
"#,
    );
    let (removed, _) = requests(
        r#"
"fs.read" = { paths = ["$WORKSPACE/a", "$WORKSPACE/b"] }
"#,
    );
    assert_eq!(first.request_hash(), second.request_hash());
    assert_ne!(first.request_hash(), wider.request_hash());
    assert_ne!(first.request_hash(), added.request_hash());
    assert_ne!(first.request_hash(), removed.request_hash());
    assert_eq!(
        first.canonical_json(),
        r#"{"capabilities":{"editor.read":true,"fs.read":{"paths":["$WORKSPACE/a","$WORKSPACE/b"]}}}"#
    );
}

#[test]
fn normalizes_bracketed_ipv6_hosts() {
    let (requests, warnings) = requests(r#""net.connect" = { hosts = ["[::1]:443"] }"#);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        requests.get(Capability::NetConnect),
        Some(&Scope::Hosts(vec!["[::1]:443".to_string()]))
    );
}

#[test]
fn rejects_malformed_bracketed_hosts() {
    let (requests, warnings) = requests(
        r#"
"net.connect" = { hosts = ["[[]]:443", "[example.com]:443", "example.com]:443"] }
"#,
    );
    assert!(!requests.contains(Capability::NetConnect));
    assert_eq!(warnings.len(), 1);
}

#[test]
fn rejects_hosts_without_ports_and_zero_ports() {
    let (requests, warnings) = requests(
        r#"
"net.connect" = { hosts = ["example.com", "example.com:0", ":443", "[::1]"] }
"#,
    );
    assert!(!requests.contains(Capability::NetConnect));
    assert_eq!(warnings.len(), 1);
}

#[test]
fn rejects_unknown_path_variables_and_unbracketed_ipv6() {
    let (requests, warnings) = requests(
        r#"
"fs.read" = { paths = ["$UNKNOWN/src"] }
"net.connect" = { hosts = ["::1:443"] }
"editor.read" = true
"#,
    );
    assert!(requests.contains(Capability::EditorRead));
    assert_eq!(warnings.len(), 2);
}

#[test]
fn host_method_table_is_exhaustive_and_advertisement_matches_enforcement() {
    for method in crate::protocol::HOST_METHODS {
        assert_ne!(host_method_capability(method), MethodCapability::Unknown);
    }
    let registry = CapabilityRegistry::default();
    let mut granted = std::collections::BTreeMap::new();
    granted.insert(Capability::UiMessage, Scope::Unscoped);
    registry.replace(std::collections::BTreeMap::from([(
        "plugin".to_string(),
        EffectiveCapabilities { granted },
    )]));
    let advertised = advertised_host_methods(&registry, "plugin");
    for method in crate::protocol::HOST_METHODS {
        assert_eq!(
            advertised.contains(&method),
            is_host_method_allowed(&registry, "plugin", method)
        );
        assert_eq!(
            is_host_method_allowed(&registry, "plugin", method),
            method == "ui.message"
        );
    }
    let advertised_events = advertised_events(&registry, "plugin");
    for event in crate::protocol::EVENT_NAMES {
        assert_eq!(
            advertised_events.contains(&event),
            is_event_allowed(&registry, "plugin", event)
        );
        assert!(!advertised_events.contains(&event));
    }
}

#[test]
fn editor_command_scopes_intersect_and_cover() {
    let outer = Scope::EditorCommands(vec!["edit.format".to_string(), "file.save".to_string()]);
    let inner = Scope::EditorCommands(vec!["file.save".to_string()]);
    assert!(outer.covers(&inner));
    assert!(!inner.covers(&outer));
    assert_eq!(intersect_scope(&outer, &inner), Some(inner));
    assert_eq!(
        outer.to_json_object().get("commands"),
        Some(&serde_json::json!(["edit.format", "file.save"]))
    );
}

#[test]
fn component_hash_changes_the_request_hash() {
    let (base, _) = requests(r#""editor.read" = true"#);
    let mut first = base.clone();
    first.set_artifact_hash("sha256:first");
    let mut second = base.clone();
    second.set_artifact_hash("sha256:second");
    assert_ne!(base.request_hash(), first.request_hash());
    assert_ne!(first.request_hash(), second.request_hash());
    assert!(first
        .canonical_json()
        .contains("\"component\":\"sha256:first\""));
}
