use super::*;
use crate::protocol::{
    CAPABILITY_DENIED, INVALID_PARAMS, MAX_JOB_ARGUMENT_BYTES, MAX_JOB_ARGUMENT_TOTAL_BYTES,
};

#[test]
fn rejects_a_command_outside_the_scope() {
    let (manager, _receiver) = manager("scoped", &["/bin/sh"]);
    let error = manager
        .open(&plugin("scoped"), params("true", &[]))
        .unwrap_err();
    assert_eq!(error.code, CAPABILITY_DENIED);
}

#[test]
fn rejects_a_cwd_outside_the_workspace() {
    let (manager, _receiver) = manager("cwd", &["/bin/sh"]);
    let mut params = params("/bin/sh", &[]);
    params.cwd = Some("/".to_string());
    let error = manager.open(&plugin("cwd"), params).unwrap_err();
    assert_eq!(error.code, INVALID_PARAMS);
}

#[test]
fn rejects_oversized_argument_total() {
    let (manager, _receiver) = manager("args", &["/bin/sh"]);
    let mut params = params("/bin/sh", &[]);
    let count = MAX_JOB_ARGUMENT_TOTAL_BYTES / MAX_JOB_ARGUMENT_BYTES + 1;
    params.args = vec!["x".repeat(MAX_JOB_ARGUMENT_BYTES); count];
    let error = manager.open(&plugin("args"), params).unwrap_err();
    assert_eq!(error.code, INVALID_PARAMS);
}

#[test]
fn rejects_invalid_environment() {
    let (manager, _receiver) = manager("env", &["/bin/sh"]);
    for (key, value) in [
        ("", "value"),
        ("INVALID=KEY", "value"),
        ("KEY\0", "value"),
        ("KEY", "value\0"),
    ] {
        let mut params = params("/bin/sh", &[]);
        params.env.insert(key.to_string(), value.to_string());
        let error = manager.open(&plugin("env"), params).unwrap_err();
        assert_eq!(error.code, INVALID_PARAMS, "{key}");
    }
}

#[test]
fn forwards_environment() {
    let (manager, receiver) = manager("env-pass", &["/bin/sh"]);
    let mut params = params("/bin/sh", &["-c", "printf \"$KAWI_JOB_TEST\""]);
    params
        .env
        .insert("KAWI_JOB_TEST".to_string(), "forwarded".to_string());
    let job = manager.open(&plugin("env-pass"), params).expect("open").job;
    assert_eq!(output_text(&receiver, job), "forwarded");
    wait_exit(&receiver, job);
}
