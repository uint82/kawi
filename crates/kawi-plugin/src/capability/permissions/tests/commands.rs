use super::super::*;
use super::support::*;

#[test]
fn command_grants_compare_canonical_paths() {
    let permissions = format!(
        r#"
[plugins.plugin.grant]
"process.spawn" = {{ commands = ["{}"] }}
"#,
        canonical_command("sh")
    );
    let granted = effective_for(r#""process.spawn" = { commands = ["sh"] }"#, &permissions);
    assert_eq!(
        granted.scope(Capability::ProcessSpawn),
        Some(&Scope::Commands(vec![canonical_command("sh")]))
    );
    assert!(!granted
        .warnings
        .iter()
        .any(|warning| warning.contains("partially")));
}

#[test]
fn command_grants_match_bare_names_and_absolute_paths() {
    let absolute = canonical_command("sh");
    let permissions = format!(
        r#"
[plugins.plugin.grant]
"process.spawn" = {{ commands = ["{absolute}"] }}
"#
    );
    let forward = effective_for(r#""process.spawn" = { commands = ["sh"] }"#, &permissions);
    assert_eq!(
        forward.scope(Capability::ProcessSpawn),
        Some(&Scope::Commands(vec![absolute.clone()]))
    );

    let request = format!(r#""process.spawn" = {{ commands = ["{absolute}"] }}"#);
    let reverse = effective_for(
        &request,
        r#"
[plugins.plugin.grant]
"process.spawn" = { commands = ["sh"] }
"#,
    );
    assert_eq!(
        reverse.scope(Capability::ProcessSpawn),
        Some(&Scope::Commands(vec![absolute]))
    );
}

#[test]
fn command_grants_require_the_same_canonical_path() {
    let granted = effective_for(
        r#""process.spawn" = { commands = ["sh"] }"#,
        r#"
[plugins.plugin.grant]
"process.spawn" = { commands = ["true"] }
"#,
    );
    assert!(!granted.has(Capability::ProcessSpawn));
}

#[test]
fn unresolved_commands_are_not_granted() {
    let granted = effective_for(
        r#""process.spawn" = { commands = ["/does/not/exist"] }"#,
        r#"
[plugins.plugin.grant]
"process.spawn" = { commands = ["/does/not/exist"] }
"#,
    );
    assert!(!granted.has(Capability::ProcessSpawn));
}

#[test]
fn relative_commands_are_rejected() {
    let requests = requests(r#""process.spawn" = { commands = ["./tool"] }"#);
    assert!(!requests.contains(Capability::ProcessSpawn));

    let granted = effective_for(
        r#""process.spawn" = { commands = ["sh"] }"#,
        r#"
[plugins.plugin.grant]
"process.spawn" = { commands = ["scripts/tool"] }
"#,
    );
    assert!(!granted.has(Capability::ProcessSpawn));
}

#[test]
fn equivalent_request_spellings_are_not_partial() {
    let absolute = canonical_command("sh");
    let request = format!(r#""process.spawn" = {{ commands = ["sh", "{absolute}"] }}"#);
    let granted = effective_for(
        &request,
        r#"
[plugins.plugin.grant]
"process.spawn" = { commands = ["sh"] }
"#,
    );
    assert_eq!(
        granted.scope(Capability::ProcessSpawn),
        Some(&Scope::Commands(vec![absolute]))
    );
    assert!(!granted
        .warnings
        .iter()
        .any(|warning| warning.contains("partially")));
}

#[test]
fn partial_grants_warn_but_still_grant_the_subset() {
    let permissions = format!(
        r#"
[plugins.plugin.grant]
"process.spawn" = {{ commands = ["{}"] }}
"#,
        canonical_command("sh")
    );
    let granted = effective_for(
        r#""process.spawn" = { commands = ["sh", "true"] }"#,
        &permissions,
    );
    assert_eq!(
        granted.scope(Capability::ProcessSpawn),
        Some(&Scope::Commands(vec![canonical_command("sh")]))
    );
    assert!(granted
        .warnings
        .iter()
        .any(|warning| warning.contains("partially")));
}
