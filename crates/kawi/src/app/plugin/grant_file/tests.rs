use super::*;
use kawi_plugin::capability::parse_capability_requests;

fn requests(text: &str) -> CapabilityRequests {
    let value: toml::Value = toml::from_str(text).expect("toml");
    parse_capability_requests(value.as_table().expect("table")).0
}

fn temp_file() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("kawi-grant-file-{}-{id}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir.join("permissions.toml")
}

#[test]
fn merges_grants_and_preserves_comments() {
    let path = temp_file();
    fs::write(
        &path,
        "# keep\n[plugins.other.grant]\n\"ui.message\" = true\n",
    )
    .unwrap();
    let requests = requests(r#""fs.read" = { paths = ["$WORKSPACE/src"] }"#);
    write_grants(
        &path,
        "sample",
        &requests,
        &requests.request_hash(),
        &BTreeMap::new(),
        &[GrantDecision {
            capability: Capability::FsRead,
            allow: true,
        }],
    )
    .unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("# keep"));
    assert!(text.contains("[plugins.other.grant]"));
    assert!(text.contains("[plugins.sample]"));
    assert!(text.contains("[plugins.sample.approved]"));
    assert!(text.contains("request_hash"));
    assert!(text.contains("\"fs.read\" = { paths = [\"$WORKSPACE/src\"] }"));
}

#[test]
fn concurrent_edits_are_re_read_before_merge() {
    let path = temp_file();
    let first_requests = requests(r#""editor.read" = true"#);
    write_grants(
        &path,
        "sample",
        &first_requests,
        &first_requests.request_hash(),
        &BTreeMap::new(),
        &[GrantDecision {
            capability: Capability::EditorRead,
            allow: true,
        }],
    )
    .unwrap();
    fs::write(
        &path,
        format!(
            "{}\n[plugins.other.grant]\n\"ui.message\" = true\n",
            fs::read_to_string(&path).unwrap()
        ),
    )
    .unwrap();
    let second_requests = requests(
        r#""editor.read" = true
"editor.write" = true
"#,
    );
    write_grants(
        &path,
        "sample",
        &second_requests,
        &second_requests.request_hash(),
        &BTreeMap::new(),
        &[GrantDecision {
            capability: Capability::EditorWrite,
            allow: true,
        }],
    )
    .unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("[plugins.other.grant]"));
    assert!(text.contains("\"editor.read\" = true"));
    assert!(text.contains("\"editor.write\" = true"));
    assert!(text.contains("[plugins.sample.approved]"));
}

#[test]
fn records_command_gates_with_the_approval() {
    let path = temp_file();
    let mut requests = requests(r#""editor.write" = true"#);
    requests.set_command_gates(BTreeMap::from([(
        "run".to_string(),
        std::collections::BTreeSet::from([Capability::EditorWrite]),
    )]));
    write_grants(
        &path,
        "sample",
        &requests,
        &requests.request_hash(),
        &BTreeMap::new(),
        &[GrantDecision {
            capability: Capability::EditorWrite,
            allow: true,
        }],
    )
    .unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("[plugins.sample.command_gates]"));
    assert!(text.contains("run = [\"editor.write\"]"));
    let parsed = kawi_plugin::capability::parse_permissions(&text).unwrap();
    assert_eq!(
        parsed.plugins["sample"].command_gates.as_ref().unwrap()["run"],
        ["editor.write"]
    );
}
