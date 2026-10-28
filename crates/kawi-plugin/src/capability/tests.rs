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
"fs.read" = { paths = ["$WORKSPACE/src", "$WORKSPACE/src"] }
"process.spawn" = { commands = ["git"] }
"net.connect" = { hosts = ["API.EXAMPLE.COM:443"] }
"#,
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(requests.get(Capability::EditorRead), Some(&Scope::Unscoped));
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
