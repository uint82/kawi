use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{json, Value};

use kawi_plugin::{load_catalog, Capability, PluginHost, PluginHostEvent, PluginStatus};

use super::common::*;

#[test]
fn invokes_a_command_and_returns_the_result() {
    let host = TestHost::start("normal", 1_000);
    let request = host.invoke();
    let result = host.finish(request).unwrap();
    assert!(result["echo"].is_object());
}

#[test]
fn serves_host_requests_during_an_invocation() {
    let host = TestHost::start("host-call", 1_000);
    let request = host.invoke();
    let (id, method, _) = host.host_request();
    assert_eq!(method, "editor.get_state");
    host.host
        .respond("fake", id, Ok(json!({"revision": 7})))
        .unwrap();
    let result = host.finish(request).unwrap();
    assert_eq!(result["host_result"], json!({"revision": 7}));
}

#[test]
fn rejects_garbage_on_stdout() {
    let host = TestHost::start("garbage", 300);
    let request = host.invoke();
    let error = host.finish(request).unwrap_err();
    assert!(matches!(error, kawi_plugin::PluginError::Protocol(_)));
}

#[test]
fn rejects_oversized_messages() {
    let host = TestHost::start("oversized", 300);
    let request = host.invoke();
    let error = host.finish(request).unwrap_err();
    assert!(matches!(error, kawi_plugin::PluginError::Protocol(_)));
}

#[test]
fn times_out_a_plugin_that_never_initializes() {
    let host = TestHost::start("hang-init", 100);
    let request = host.invoke();
    let error = host.finish(request).unwrap_err();
    assert_eq!(error, kawi_plugin::PluginError::Timeout);
}

#[test]
fn reports_a_process_exit_mid_request() {
    let host = TestHost::start("exit-mid", 1_000);
    let request = host.invoke();
    let error = host.finish(request).unwrap_err();
    assert_eq!(error, kawi_plugin::PluginError::ProcessExited);
}

#[test]
fn backs_off_after_a_crash() {
    let host = TestHost::start("exit-mid", 1_000);
    let first = host.invoke();
    assert_eq!(
        host.finish(first).unwrap_err(),
        kawi_plugin::PluginError::ProcessExited
    );
    let second = host.invoke();
    let error = host.finish(second).unwrap_err();
    assert!(matches!(error, kawi_plugin::PluginError::Backoff { .. }));
    assert!(matches!(
        host.host.status("fake"),
        Some(PluginStatus::Backoff { .. })
    ));
}

#[test]
fn recovers_after_an_invocation_timeout() {
    let host = TestHost::start("hang-invoke-once", 100);
    let first = host.invoke();
    assert_eq!(
        host.finish(first).unwrap_err(),
        kawi_plugin::PluginError::Timeout
    );
    let second = host.invoke();
    assert!(matches!(
        host.finish(second).unwrap_err(),
        kawi_plugin::PluginError::Backoff { .. }
    ));
    wait_for_retry(&host);
    let third = host.invoke();
    let result = host.finish(third).unwrap();
    assert!(result["echo"].is_object());
}

#[test]
fn successful_invocation_resets_the_crash_counter() {
    let host = TestHost::start("crash-once", 1_000);
    let first = host.invoke();
    assert_eq!(
        host.finish(first).unwrap_err(),
        kawi_plugin::PluginError::ProcessExited
    );
    wait_for_retry(&host);
    let second = host.invoke();
    assert!(host.finish(second).is_ok());
    let third = host.invoke();
    assert!(host.finish(third).is_ok());
    assert_eq!(host.host.status("fake"), Some(PluginStatus::Ready));
}

#[test]
fn restart_recovers_a_disabled_plugin() {
    let host = TestHost::start("crash-five", 1_000);
    for attempt in 0..5 {
        let request = host.invoke();
        assert_eq!(
            host.finish(request).unwrap_err(),
            kawi_plugin::PluginError::ProcessExited,
            "attempt {attempt}"
        );
        if attempt < 4 {
            wait_for_retry(&host);
        }
    }
    assert_eq!(host.host.status("fake"), Some(PluginStatus::Disabled));
    host.host.restart("fake").unwrap();
    let request = host.invoke();
    assert!(host.finish(request).is_ok());
}

#[test]
fn runs_the_example_plugin_as_a_real_process() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/echo");
    let catalog = load_catalog(root.parent().expect("example plugin parent"));
    assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
    let (event_tx, events) = mpsc::channel();
    let (wake, _wake_rx) = mpsc::channel();
    let host = PluginHost::start(
        &catalog,
        registry_for("echo", &[Capability::UiMessage]),
        event_tx,
        wake,
    );
    host.invoke("echo", "ping", json!({"name": "kawi"}), context())
        .unwrap();
    let result = loop {
        match events.recv_timeout(Duration::from_secs(5)).expect("event") {
            PluginHostEvent::InvocationFinished { result, .. } => break result,
            _ => continue,
        }
    };
    assert_eq!(result.unwrap(), json!({"message": "pong kawi"}));
    host.shutdown();
}

#[test]
fn redacts_invocation_context_without_editor_read() {
    let host = TestHost::start_with("normal", 5_000, registry_without_editor_read());
    let request = host.invoke_with(json!({"name": "command input"}));
    let result = host.finish(request).expect("result");
    assert_eq!(result["echo"]["args"]["name"], "command input");
    let context = &result["echo"]["context"];
    assert_eq!(context["buffer"], 0);
    assert_eq!(context["revision"], 0);
    assert_eq!(context["path"], Value::Null);
    assert_eq!(context["language"], Value::Null);
    assert_eq!(context["mode"], "normal");
    assert_eq!(context["cursor"]["line"], 0);
    assert_eq!(context["cursor"]["byte_column"], 0);
    assert_eq!(context["selection"], Value::Null);
    assert!(context.get("cwd").is_none());
    assert!(context.get("workspace").is_none());
}

#[test]
fn preserves_invocation_context_with_editor_read() {
    let host = TestHost::start("normal", 5_000);
    let request = host.invoke_with(json!({"name": "visible"}));
    let result = host.finish(request).expect("result");
    assert_eq!(result["echo"]["args"]["name"], "visible");
    let context = &result["echo"]["context"];
    assert_eq!(context["buffer"], 1);
    assert_eq!(context["revision"], 7);
    assert_eq!(context["path"], "/tmp/test.rs");
}
