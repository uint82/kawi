use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_kawi"))
        .args(args)
        .output()
        .expect("run kawi plugin api")
}

#[test]
fn json_output_reports_the_registered_api() {
    let output = run(&["plugin", "api", "--json"]);
    assert!(output.status.success(), "{:?}", output.stderr);

    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("api JSON");
    assert_eq!(report["version"]["protocol"], 1);
    assert!(report["methods"].as_array().unwrap().len() >= 74);
    assert_eq!(report["events"].as_array().unwrap().len(), 19);

    let state = report["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "editor.get_state")
        .expect("editor.get_state entry");
    assert_eq!(state["signature"]["returns"], "GetStateResult");
    assert_eq!(state["signature"]["parameters"][0]["name"], "include_text");
    assert_eq!(state["signature"]["parameters"][0]["required"], false);
    assert!(report["events"][0].get("signature").is_none());
}

#[test]
fn text_output_lists_methods_and_events() {
    let output = run(&["plugin", "api"]);
    assert!(output.status.success(), "{:?}", output.stderr);

    let text = String::from_utf8(output.stdout).expect("utf-8 output");
    assert!(text.contains("Host methods (74):"));
    assert!(text.contains("Events (19):"));
    assert!(text.contains("editor.get_state"));
    assert!(text.contains("(include_text?: boolean) -> GetStateResult"));
}

#[test]
fn unknown_flags_fail_with_usage() {
    let output = run(&["plugin", "api", "--yaml"]);
    assert!(!output.status.success());

    let error = String::from_utf8(output.stderr).expect("utf-8 error");
    assert!(error.contains("usage: kawi plugin api [--json]"));
}
