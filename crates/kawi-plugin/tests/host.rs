use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

use kawi_plugin::{
    load_catalog, InvocationContext, NotifyOutcome, PluginHost, PluginHostEvent, PluginStatus,
    Position,
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TestHost {
    host: PluginHost,
    events: Receiver<PluginHostEvent>,
    _wake: Receiver<()>,
    root: PathBuf,
}

impl TestHost {
    fn start(mode: &str, timeout_ms: u64) -> Self {
        let root = temp_dir();
        let executable = env!("CARGO_BIN_EXE_fake-plugin");
        write_manifest(&root, "fake", executable, mode, timeout_ms);
        let catalog = load_catalog(&root);
        assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
        let (event_tx, events) = mpsc::channel();
        let (wake, wake_rx) = mpsc::channel();
        let host = PluginHost::start(&catalog, event_tx, wake);
        Self {
            host,
            events,
            _wake: wake_rx,
            root,
        }
    }

    fn invoke(&self) -> u64 {
        self.host
            .invoke("fake", "ping", Value::Null, context())
            .expect("invoke")
    }

    fn finish(&self, request: u64) -> Result<Value, kawi_plugin::PluginError> {
        loop {
            match self
                .events
                .recv_timeout(Duration::from_secs(3))
                .expect("event")
            {
                PluginHostEvent::InvocationFinished {
                    request: finished,
                    result,
                    ..
                } if finished == request => return result,
                _ => continue,
            }
        }
    }

    fn host_request(&self) -> (Value, String, Value) {
        loop {
            match self
                .events
                .recv_timeout(Duration::from_secs(3))
                .expect("event")
            {
                PluginHostEvent::HostRequest {
                    id, method, params, ..
                } => return (id, method, params),
                _ => continue,
            }
        }
    }
}

impl Drop for TestHost {
    fn drop(&mut self) {
        self.host.shutdown();
        fs::remove_dir_all(&self.root).ok();
    }
}

fn temp_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("kawi-plugin-host-{}-{id}", std::process::id()));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_manifest(root: &Path, name: &str, executable: &str, mode: &str, timeout_ms: u64) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let manifest = format!(
        "name = \"{name}\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [{executable:?}, {mode:?}]\ntimeout_ms = {timeout_ms}\nsubscribes = [\"buffer_changed\", \"file_saved\", \"mode_changed\", \"unknown_event\"]\n\n[[commands]]\nid = \"ping\"\ntitle = \"Ping\"\n"
    );
    fs::write(dir.join("plugin.toml"), manifest).unwrap();
}

fn context() -> InvocationContext {
    InvocationContext {
        buffer: 1,
        revision: 7,
        path: Some("/tmp/test.rs".to_string()),
        language: Some("rust".to_string()),
        mode: "normal".to_string(),
        cursor: Position {
            line: 0,
            byte_column: 0,
        },
        selection: None,
    }
}

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

fn wait_for_retry(host: &TestHost) {
    if let Some(PluginStatus::Backoff { remaining_ms }) = host.host.status("fake") {
        std::thread::sleep(Duration::from_millis(remaining_ms.saturating_add(50)));
    }
}

#[test]
fn unknown_event_subscriptions_are_ignored() {
    let host = TestHost::start("normal", 1_000);
    assert_eq!(
        host.host
            .notify("fake", "unknown_event", json!({"buffer": 1}))
            .unwrap(),
        NotifyOutcome::NotSubscribed
    );
    assert_eq!(
        host.host
            .notify("fake", "buffer_changed", json!({"buffer": 1}))
            .unwrap(),
        NotifyOutcome::NotRunning
    );
}

#[test]
fn events_flow_while_an_invocation_is_running() {
    let host = TestHost::start("host-call", 1_000);
    let request = host.invoke();
    let (id, _, _) = host.host_request();
    assert_eq!(
        host.host
            .notify(
                "fake",
                "buffer_changed",
                json!({"buffer": 1, "revision": 2})
            )
            .unwrap(),
        NotifyOutcome::Queued
    );
    host.host
        .respond("fake", id, Ok(json!({"revision": 1})))
        .unwrap();
    assert!(host.finish(request).is_ok());
}

#[test]
fn event_flood_against_a_stalled_plugin_drops_without_blocking() {
    let host = TestHost::start("stalled", 1_000);
    host.host.activate("fake").unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while host.host.status("fake") != Some(PluginStatus::Ready)
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(host.host.status("fake"), Some(PluginStatus::Ready));
    let started = std::time::Instant::now();
    let mut overflow = 0;
    for revision in 0..10_000 {
        match host
            .host
            .notify("fake", "buffer_changed", json!({"revision": revision}))
            .unwrap()
        {
            NotifyOutcome::Overflow => overflow += 1,
            _ => {}
        }
    }
    assert!(overflow > 0);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn activation_starts_a_plugin_without_a_command() {
    let root = temp_dir();
    let executable = env!("CARGO_BIN_EXE_fake-plugin");
    let dir = root.join("fake");
    fs::create_dir_all(&dir).unwrap();
    let manifest = format!(
        "name = \"fake\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [{executable:?}, \"normal\" ]\ntimeout_ms = 1000\n\n[[activation_events]]\nevent = \"on_startup\"\n\n[[commands]]\nid = \"ping\"\ntitle = \"Ping\"\n"
    );
    fs::write(dir.join("plugin.toml"), manifest).unwrap();
    let catalog = load_catalog(&root);
    assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
    let (event_tx, _events) = mpsc::channel();
    let (wake, _wake_rx) = mpsc::channel();
    let host = PluginHost::start(&catalog, event_tx, wake);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while host.status("fake") != Some(PluginStatus::Ready) && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(host.status("fake"), Some(PluginStatus::Ready));
    host.shutdown();
    fs::remove_dir_all(root).ok();
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
    let host = PluginHost::start(&catalog, event_tx, wake);
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
