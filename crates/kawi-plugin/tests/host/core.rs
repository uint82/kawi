use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use std::collections::BTreeMap;

use kawi_plugin::{
    load_catalog, Capability, EffectiveCapabilities, HostEnvironment, PluginHost, PluginHostEvent,
    PluginStatus, Scope,
};

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
fn restart_cancels_an_in_flight_invocation() {
    let host = TestHost::start("hang-invoke", 5_000);
    let request = host.invoke();
    let started = Instant::now();
    host.host.restart("fake").unwrap();
    let result = host.finish(request);
    assert_eq!(result.unwrap_err(), kawi_plugin::PluginError::Restart);
    assert!(started.elapsed() < Duration::from_secs(2));
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
    let environment = HostEnvironment::from_paths(Some(root.parent().unwrap().to_path_buf()), None);
    let host = PluginHost::start(
        &catalog,
        registry_for("echo", &[Capability::UiMessage]),
        environment,
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

#[test]
fn landlock_limits_filesystem_reads_to_the_granted_scope() {
    if !kawi_process::sandbox_status().is_enforced() {
        return;
    }
    let denied = TestHost::start_with("read-file", 5_000, registry());
    let secret = denied.root.join("secret.txt");
    std::fs::write(&secret, "outside").unwrap();
    let request = denied.invoke_with(json!({"path": secret}));
    let result = denied.finish(request).expect("result");
    assert!(result.get("error").is_some(), "{result}");

    let allowed = TestHost::start_with("read-file", 5_000, registry_with_fs_read());
    let secret = allowed.root.join("secret.txt");
    std::fs::write(&secret, "outside").unwrap();
    let request = allowed.invoke_with(json!({"path": secret}));
    let result = allowed.finish(request).expect("result");
    assert_eq!(result["content"], "outside", "{result}");
}

#[test]
fn landlock_limits_process_execution_to_the_granted_scope() {
    if !kawi_process::sandbox_status().is_enforced() {
        return;
    }
    let denied = TestHost::start_with("exec", 5_000, registry());
    let request = denied.invoke_with(json!({"path": "/bin/true"}));
    let result = denied.finish(request).expect("result");
    assert!(result.get("error").is_some(), "{result}");

    let allowed = TestHost::start_with("exec", 5_000, registry_with_process_spawn());
    let request = allowed.invoke_with(json!({"path": "/bin/true"}));
    let result = allowed.finish(request).expect("result");
    assert_eq!(result["success"], true, "{result}");
}

fn registry_with_process_spawn() -> kawi_plugin::CapabilityRegistry {
    registry_with_scope(
        "fake",
        Capability::ProcessSpawn,
        Scope::Commands(vec!["/bin/true".to_string()]),
    )
}

fn registry_with_fs_read() -> kawi_plugin::CapabilityRegistry {
    registry_with_scope(
        "fake",
        Capability::FsRead,
        Scope::Paths(vec!["$WORKSPACE".to_string()]),
    )
}

fn registry_with_scope(
    plugin: &str,
    capability: Capability,
    scope: Scope,
) -> kawi_plugin::CapabilityRegistry {
    let registry = kawi_plugin::CapabilityRegistry::default();
    registry.replace(BTreeMap::from([(
        plugin.to_string(),
        EffectiveCapabilities {
            granted: BTreeMap::from([(capability, scope)]),
            ..EffectiveCapabilities::default()
        },
    )]));
    registry
}

fn start_python_probe(capabilities: kawi_plugin::CapabilityRegistry) -> TestHost {
    let root = temp_dir();
    let dir = root.join("python-probe");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("sandbox_probe.py"),
        include_str!("sandbox_probe.py"),
    )
    .unwrap();
    std::fs::write(
        dir.join("plugin.toml"),
        r#"name = "python-probe"
version = "0.1.0"
protocol = 1
command = ["python3", "${plugin_dir}/sandbox_probe.py"]
timeout_ms = 5000

[[commands]]
id = "probe"
title = "Probe"
"#,
    )
    .unwrap();
    TestHost::start_custom(root, capabilities)
}

fn python_probe(
    capabilities: kawi_plugin::CapabilityRegistry,
    args: Value,
) -> (Value, Vec<String>) {
    let host = start_python_probe(capabilities);
    let request = host.invoke_as("python-probe", args);
    let (result, logs) = host.finish_with_logs(request);
    (result.expect("result"), logs)
}

#[test]
fn python_plugin_cannot_read_unlisted_paths() {
    if !kawi_process::sandbox_status().is_enforced() || !python_available() {
        return;
    }
    let (result, _) = python_probe(
        registry_for("python-probe", &[]),
        json!({"operation": "read", "path": "/etc/passwd"}),
    );
    assert_eq!(result["ok"], false, "{result}");
    assert!(result["error"]
        .as_str()
        .is_some_and(|error| error.contains("Permission denied")));
}

#[test]
fn python_plugin_with_read_scope_cannot_write() {
    if !kawi_process::sandbox_status().is_enforced() || !python_available() {
        return;
    }
    let host = start_python_probe(registry_with_scope(
        "python-probe",
        Capability::FsRead,
        Scope::Paths(vec!["$WORKSPACE/granted".to_string()]),
    ));
    std::fs::create_dir_all(host.root.join("granted")).unwrap();
    std::fs::write(host.root.join("granted/public.txt"), "public").unwrap();
    let (read, _) = host.finish_with_logs(host.invoke_as(
        "python-probe",
        json!({"operation": "read", "path": host.root.join("granted/public.txt")}),
    ));
    assert_eq!(read.expect("read")["content"], "public");
    let (write, _) = host.finish_with_logs(host.invoke_as(
        "python-probe",
        json!({"operation": "write", "path": host.root.join("granted/out.txt")}),
    ));
    let write = write.expect("write");
    assert_eq!(write["ok"], false, "{write}");
    assert!(write["error"]
        .as_str()
        .is_some_and(|error| error.contains("Permission denied")));
}

#[test]
#[cfg(unix)]
fn python_plugin_cannot_follow_symlinks_out_of_a_granted_directory() {
    if !kawi_process::sandbox_status().is_enforced() || !python_available() {
        return;
    }
    let host = start_python_probe(registry_with_scope(
        "python-probe",
        Capability::FsRead,
        Scope::Paths(vec!["$WORKSPACE/granted".to_string()]),
    ));
    let granted = host.root.join("granted");
    let secret = host.root.join("secret");
    std::fs::create_dir_all(&granted).unwrap();
    std::fs::create_dir_all(&secret).unwrap();
    std::fs::write(secret.join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(secret.join("secret.txt"), granted.join("link")).unwrap();
    let (result, _) = python_probe_at(
        &host,
        json!({"operation": "read", "path": granted.join("link")}),
    );
    assert_eq!(result["ok"], false, "{result}");
    assert!(result["error"]
        .as_str()
        .is_some_and(|error| error.contains("Permission denied")));
}

fn python_probe_at(host: &TestHost, args: Value) -> (Value, Vec<String>) {
    let (result, logs) = host.finish_with_logs(host.invoke_as("python-probe", args));
    (result.expect("result"), logs)
}

#[test]
fn skipped_grant_paths_are_logged() {
    if !kawi_process::sandbox_status().is_enforced() || !python_available() {
        return;
    }
    let (result, logs) = python_probe(
        registry_with_scope(
            "python-probe",
            Capability::FsRead,
            Scope::Paths(vec!["$WORKSPACE/missing".to_string()]),
        ),
        json!({"operation": "read", "path": "/etc/passwd"}),
    );
    assert_eq!(result["ok"], false, "{result}");
    assert!(
        logs.iter()
            .any(|line| line.contains("skipped granted fs.read path")
                && line.contains("does not exist")),
        "{logs:?}"
    );
}

fn python_available() -> bool {
    std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_ok()
}

#[test]
#[cfg(unix)]
fn python_runtime_prefix_is_readable_inside_the_sandbox() {
    if !kawi_process::sandbox_status().is_enforced() || !python_available() {
        return;
    }
    let python = kawi_process::find_executable("python3").expect("python3");
    let root = temp_dir();
    let runtime = root.join("runtime");
    std::fs::create_dir_all(runtime.join("bin")).unwrap();
    std::os::unix::fs::symlink(&python, runtime.join("bin/python")).unwrap();
    std::fs::write(
        runtime.join("pyvenv.cfg"),
        format!(
            "home = {}\ninclude-system-site-packages = false\n",
            python.parent().unwrap().display()
        ),
    )
    .unwrap();
    let dir = root.join("python-probe");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("sandbox_probe.py"),
        include_str!("sandbox_probe.py"),
    )
    .unwrap();
    std::fs::write(
        dir.join("plugin.toml"),
        format!(
            r#"name = "python-probe"
version = "0.1.0"
protocol = 1
command = ["{}", "${{plugin_dir}}/sandbox_probe.py"]
timeout_ms = 5000

[[commands]]
id = "probe"
title = "Probe"
"#,
            runtime.join("bin/python").display()
        ),
    )
    .unwrap();
    let host = TestHost::start_custom(root, registry_for("python-probe", &[Capability::FsRead]));
    let (result, _) =
        host.finish_with_logs(host.invoke_as("python-probe", json!({"operation": "prefix"})));
    let result = result.expect("result");
    assert_eq!(
        result["prefix"].as_str(),
        Some(runtime.to_str().expect("utf-8"))
    );
}

#[test]
fn sandbox_startup_failure_is_logged_with_a_reason() {
    if !kawi_process::sandbox_status().is_enforced() {
        return;
    }
    let root = temp_dir();
    let dir = root.join("missing");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("plugin.toml"),
        r#"name = "missing"
version = "0.1.0"
protocol = 1
command = ["/does/not/exist"]
timeout_ms = 1000

[[commands]]
id = "probe"
title = "Probe"
"#,
    )
    .unwrap();
    let host = TestHost::start_custom(root, registry_for("missing", &[]));
    let (result, logs) = host.finish_with_logs(host.invoke_as("missing", Value::Null));
    let error = result.expect_err("spawn should fail");
    assert!(error.to_string().contains("landlock"), "{error}");
    assert!(
        logs.iter().any(|line| line.contains("landlock")),
        "{logs:?}"
    );
}
