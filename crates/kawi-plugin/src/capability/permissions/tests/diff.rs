use std::collections::{BTreeMap, BTreeSet};

use super::super::*;
use super::support::*;

#[test]
fn added_request_is_withheld_until_approved() {
    let granted = effective_for(
        r#""editor.read" = true
"fs.read" = { paths = ["$WORKSPACE"] }
"#,
        r#"
[plugins.plugin.approved]
"editor.read" = true

[plugins.plugin.grant]
"editor.read" = true
"fs.read" = { paths = ["$WORKSPACE"] }
"#,
    );
    assert!(granted.has(Capability::EditorRead));
    assert_eq!(granted.missing, vec![Capability::FsRead]);
    assert_eq!(granted.changed_capabilities, vec![Capability::FsRead]);
}

#[test]
fn widened_request_never_carries_the_old_grant() {
    let granted = effective_for(
        r#""fs.read" = { paths = ["$WORKSPACE"] }"#,
        r#"
[plugins.plugin.approved]
"fs.read" = { paths = ["$WORKSPACE/src"] }

[plugins.plugin.grant]
"fs.read" = { paths = ["$WORKSPACE/src"] }
"#,
    );
    assert!(!granted.has(Capability::FsRead));
    assert_eq!(granted.missing, vec![Capability::FsRead]);
    assert_eq!(granted.changed_capabilities, vec![Capability::FsRead]);
    assert!(granted
        .changed_requests
        .iter()
        .any(|label| label == "fs.read widened"));
}

#[test]
fn narrowed_request_keeps_the_grant() {
    let granted = effective_for(
        r#""fs.read" = { paths = ["$WORKSPACE/src"] }"#,
        r#"
[plugins.plugin.approved]
"fs.read" = { paths = ["$WORKSPACE"] }

[plugins.plugin.grant]
"fs.read" = { paths = ["$WORKSPACE"] }
"#,
    );
    assert_eq!(
        granted.scope(Capability::FsRead),
        Some(&Scope::Paths(vec!["$WORKSPACE/src".to_string()]))
    );
    assert!(granted.changed_capabilities.is_empty());
    assert!(granted
        .changed_requests
        .iter()
        .any(|label| label == "fs.read narrowed"));
}

#[test]
fn removed_request_is_reported_and_grant_is_stale() {
    let granted = effective_for(
        r#""editor.read" = true"#,
        r#"
[plugins.plugin.approved]
"editor.read" = true
"fs.read" = { paths = ["$WORKSPACE"] }

[plugins.plugin.grant]
"fs.read" = { paths = ["$WORKSPACE"] }
"#,
    );
    assert!(!granted.has(Capability::FsRead));
    assert_eq!(granted.stale, vec![Capability::FsRead]);
    assert!(granted
        .changed_requests
        .iter()
        .any(|label| label == "fs.read removed"));
}

#[test]
fn reordered_request_is_unchanged() {
    let granted = effective_for(
        r#""ui.message" = true
"editor.read" = true
"#,
        r#"
[plugins.plugin.approved]
"editor.read" = true
"ui.message" = true

[plugins.plugin.grant]
"editor.read" = true
"#,
    );
    assert!(granted.has(Capability::EditorRead));
    assert!(!granted.changed);
    assert!(granted.changed_requests.is_empty());
}

#[test]
fn mixed_changes_keep_unchanged_and_narrowed_requests() {
    let granted = effective_for(
        r#"
"editor.read" = true
"fs.read" = { paths = ["$WORKSPACE/src"] }
"fs.write" = { paths = ["$HOME"] }
"process.spawn" = { commands = ["git", "rg"] }
"net.connect" = { hosts = ["example.com:443"] }
"#,
        r#"
[plugins.plugin.approved]
"editor.read" = true
"fs.read" = { paths = ["$WORKSPACE"] }
"process.spawn" = { commands = ["git"] }
"net.connect" = { hosts = ["example.com:443"] }

[plugins.plugin.grant]
"editor.read" = true
"fs.read" = { paths = ["$WORKSPACE"] }
"process.spawn" = { commands = ["git"] }
"net.connect" = { hosts = ["example.com:443"] }
"#,
    );
    assert!(granted.has(Capability::EditorRead));
    assert_eq!(
        granted.scope(Capability::FsRead),
        Some(&Scope::Paths(vec!["$WORKSPACE/src".to_string()]))
    );
    assert!(granted.has(Capability::NetConnect));
    assert!(!granted.has(Capability::ProcessSpawn));
    assert!(!granted.has(Capability::FsWrite));
    assert_eq!(
        granted.changed_capabilities,
        vec![Capability::FsWrite, Capability::ProcessSpawn]
    );
}

#[test]
fn equivalent_command_spellings_are_unchanged() {
    let absolute = canonical_command("sh");
    let permissions = format!(
        r#"
[plugins.plugin.approved]
"process.spawn" = {{ commands = ["{absolute}"] }}

[plugins.plugin.grant]
"process.spawn" = {{ commands = ["{absolute}"] }}
"#
    );
    let granted = effective_for(r#""process.spawn" = { commands = ["sh"] }"#, &permissions);
    assert!(!granted.changed);
    assert!(granted.has(Capability::ProcessSpawn));
}

#[test]
fn flipping_between_two_states_repeats_the_same_diff() {
    let approved = r#"
[plugins.plugin.approved]
"fs.read" = { paths = ["$WORKSPACE/src"] }

[plugins.plugin.grant]
"fs.read" = { paths = ["$WORKSPACE/src"] }
"#;
    let first = effective_for(r#""fs.read" = { paths = ["$WORKSPACE"] }"#, approved);
    let second = effective_for(r#""fs.read" = { paths = ["$WORKSPACE/src"] }"#, approved);
    let third = effective_for(r#""fs.read" = { paths = ["$WORKSPACE"] }"#, approved);
    assert_eq!(first.changed_requests, third.changed_requests);
    assert_eq!(first.changed_capabilities, third.changed_capabilities);
    assert!(!second.changed);
}

#[test]
fn missing_or_corrupt_approved_record_marks_everything_new() {
    let missing = raw_effective_for(
        r#""fs.read" = { paths = ["$WORKSPACE"] }"#,
        r#"
[plugins.plugin.grant]
"fs.read" = { paths = ["$WORKSPACE"] }
"#,
    );
    assert!(!missing.has(Capability::FsRead));
    assert_eq!(missing.changed_capabilities, vec![Capability::FsRead]);

    let corrupt = effective_for(
        r#""fs.read" = { paths = ["$WORKSPACE"] }"#,
        r#"
[plugins.plugin.approved]
"fs.read" = { paths = ["relative"] }

[plugins.plugin.grant]
"fs.read" = { paths = ["$WORKSPACE"] }
"#,
    );
    assert!(!corrupt.has(Capability::FsRead));
    assert_eq!(corrupt.changed_capabilities, vec![Capability::FsRead]);
    assert!(corrupt
        .warnings
        .iter()
        .any(|warning| warning.contains("approved request record is corrupt")));
}

#[test]
fn added_command_gate_is_reported() {
    let mut requests = requests(r#""editor.write" = true"#);
    requests.set_command_gates(BTreeMap::from([(
        "run".to_string(),
        BTreeSet::from([Capability::EditorWrite]),
    )]));
    let file = parse_permissions(
        r#"
[plugins.plugin.approved]
"editor.write" = true

[plugins.plugin.grant]
"editor.write" = true
"#,
    )
    .expect("permissions");
    let granted = effective(&requests, file.plugins.get("plugin"));
    assert!(granted.changed);
    assert!(granted
        .changed_requests
        .iter()
        .any(|label| label == "command run requires editor.write"));
    assert!(granted.changed_capabilities.is_empty());
}

#[test]
fn removed_command_gate_is_reported_as_widening() {
    let requests = requests(r#""editor.write" = true"#);
    let file = parse_permissions(
        r#"
[plugins.plugin.approved]
"editor.write" = true

[plugins.plugin.command_gates]
run = ["editor.write"]

[plugins.plugin.grant]
"editor.write" = true
"#,
    )
    .expect("permissions");
    let granted = effective(&requests, file.plugins.get("plugin"));
    assert!(granted.changed);
    assert!(granted
        .changed_requests
        .iter()
        .any(|label| label == "command run no longer requires editor.write"));
    assert!(granted
        .warnings
        .iter()
        .any(|warning| warning.contains("command gate removed")));
}
