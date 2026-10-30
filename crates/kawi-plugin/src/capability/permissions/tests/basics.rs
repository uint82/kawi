use super::super::*;
use super::support::*;

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

#[test]
fn partial_grants_warn_but_still_grant_the_subset() {
    let granted = effective_for(
        r#""process.spawn" = { commands = ["git", "rg"] }"#,
        r#"
[plugins.plugin.grant]
"process.spawn" = { commands = ["git"] }
"#,
    );
    assert_eq!(
        granted.scope(Capability::ProcessSpawn),
        Some(&Scope::Commands(vec!["git".to_string()]))
    );
    assert!(granted
        .warnings
        .iter()
        .any(|warning| warning.contains("partially")));
}

#[test]
fn rejects_parent_and_relative_paths_and_normalizes_trailing_slashes() {
    let granted = effective_for(
        r#""fs.read" = { paths = ["/home/u/proj/"] }"#,
        r#"
[plugins.plugin.grant]
"fs.read" = { paths = ["/home/u/proj/src/"] }
"#,
    );
    assert_eq!(
        granted.scope(Capability::FsRead),
        Some(&Scope::Paths(vec!["/home/u/proj/src".to_string()]))
    );

    let parent = requests(r#""fs.read" = { paths = ["$WORKSPACE/../.ssh"] }"#);
    assert!(!parent.contains(Capability::FsRead));

    let relative = requests(r#""fs.read" = { paths = ["proj/src"] }"#);
    assert!(!relative.contains(Capability::FsRead));
}

#[test]
fn path_subsumption_uses_components_not_string_prefixes() {
    let granted = effective_for(
        r#""fs.read" = { paths = ["/home/u/proj"] }"#,
        r#"
[plugins.plugin.grant]
"fs.read" = { paths = ["/home/u/proj-evil"] }
"#,
    );
    assert!(!granted.has(Capability::FsRead));
    assert_eq!(granted.missing, vec![Capability::FsRead]);
}

#[test]
fn command_scopes_compare_canonical_strings() {
    let granted = effective_for(
        r#""process.spawn" = { commands = ["git"] }"#,
        r#"
[plugins.plugin.grant]
"process.spawn" = { commands = ["/usr/bin/git"] }
"#,
    );
    assert!(!granted.has(Capability::ProcessSpawn));
}

#[test]
fn unknown_default_revocation_warns_and_keeps_the_default() {
    let granted = effective_for(
        r#""ui.message" = true"#,
        r#"
[plugins.plugin.grant]
"ui.mesage" = false
"#,
    );
    assert!(granted.has(Capability::UiMessage));
    assert_eq!(granted.warnings.len(), 1);
}

#[test]
fn host_wildcards_are_rejected_and_exact_hosts_do_not_suffix_match() {
    let wildcard = requests(r#""net.connect" = { hosts = ["*.example.com:443"] }"#);
    assert!(!wildcard.contains(Capability::NetConnect));

    let sibling = effective_for(
        r#""net.connect" = { hosts = ["api.example.com:443"] }"#,
        r#"
[plugins.plugin.grant]
"net.connect" = { hosts = ["evil.com:443"] }
"#,
    );
    assert!(!sibling.has(Capability::NetConnect));

    let suffix = effective_for(
        r#""net.connect" = { hosts = ["api.example.com:443"] }"#,
        r#"
[plugins.plugin.grant]
"net.connect" = { hosts = ["example.com.evil.com:443"] }
"#,
    );
    assert!(!suffix.has(Capability::NetConnect));
}

#[test]
fn request_hash_mismatch_withholds_grants_and_marks_changed() {
    let requests = requests(
        r#""editor.read" = true
"ui.message" = true
"#,
    );
    let hash = requests.request_hash();
    let permissions = format!(
        "[plugins.plugin]
request_hash = \"{hash}\"
\n[plugins.plugin.grant]\n\"editor.read\" = true\n"
    );
    let matching = effective_for(
        r#""editor.read" = true
"ui.message" = true
"#,
        &permissions,
    );
    assert!(matching.has(Capability::EditorRead));
    assert!(!matching.changed);

    let changed = effective_for(
        r#""editor.read" = true
"ui.message" = true
"#,
        "[plugins.plugin]\nrequest_hash = \"sha256:stale\"\n\n[plugins.plugin.grant]\n\"editor.read\" = true\n",
    );
    assert!(changed.changed);
    assert!(!changed.has(Capability::EditorRead));
    assert!(changed.has(Capability::UiMessage));
    assert!(changed
        .warnings
        .iter()
        .any(|warning| warning.contains("request set changed")));
}

#[test]
fn added_widened_and_shrunk_requests_change_the_hash() {
    let base = requests(r#""fs.read" = { paths = ["$WORKSPACE/src"] }"#);
    let added = requests(
        r#""fs.read" = { paths = ["$WORKSPACE/src"] }
"editor.read" = true
"#,
    );
    let widened = requests(r#""fs.read" = { paths = ["$WORKSPACE"] }"#);
    let shrunk = requests(r#""fs.read" = { paths = ["$WORKSPACE/src/main.rs"] }"#);
    let hash = base.request_hash();
    assert_ne!(hash, added.request_hash());
    assert_ne!(hash, widened.request_hash());
    assert_ne!(hash, shrunk.request_hash());
}
