use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use kawi_plugin::{
    host_method_capability, is_host_method_allowed, load_catalog, load_permission_state,
    Capability, CapabilityRegistry, HostEnvironment, InvocationContext, MethodCapability,
    NotifyOutcome, PluginCatalog, PluginError, PluginHost, PluginHostEvent, PluginStatus, Position,
    RpcError,
};

use super::scenario::{Behavior, Scenario};

const PLUGIN_NAME: &str = "conformance";
const START_TIMEOUT: Duration = Duration::from_secs(5);
const INVOCATION_TIMEOUT: Duration = Duration::from_secs(10);
const DRIVER_POLL: Duration = Duration::from_millis(50);
const STATUS_POLL: Duration = Duration::from_millis(5);
const RELEASE_POLL: Duration = Duration::from_millis(10);

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Runtime {
    Process,
    Wasm,
}

impl Runtime {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::Wasm => "wasm",
        }
    }

    pub(super) fn command(self, behavior: Behavior) -> &'static str {
        match self {
            Self::Process => behavior.process_command(),
            Self::Wasm => behavior.command(),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct HostRequestRecord {
    pub(super) method: String,
    pub(super) denied: bool,
    pub(super) responded: bool,
}

#[derive(Debug, Default, Clone)]
pub(super) struct Observation {
    pub(super) host_requests: Vec<HostRequestRecord>,
    pub(super) logs: Vec<String>,
    pub(super) statuses: Vec<PluginStatus>,
    pub(super) required: Vec<Capability>,
}

#[derive(Debug, Clone)]
pub(super) struct Finished {
    pub(super) request: u64,
    pub(super) result: Result<Value, PluginError>,
}

struct Policy {
    stale_revision: bool,
    withhold: bool,
    release: Arc<(Mutex<bool>, Condvar)>,
}

impl Policy {
    fn new() -> Self {
        Self {
            stale_revision: false,
            withhold: false,
            release: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }
}

pub(super) struct ConformanceHost {
    root: PathBuf,
    grants_path: PathBuf,
    catalog: PluginCatalog,
    host: Arc<PluginHost>,
    driver: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
    finished: Receiver<Finished>,
    observation: Arc<Mutex<Observation>>,
    registry: CapabilityRegistry,
    policy: Arc<Mutex<Policy>>,
    _wake: Receiver<()>,
}

impl ConformanceHost {
    pub(super) fn start(runtime: Runtime, scenario: &Scenario) -> Self {
        let root = temp_root();
        let config = root.join("config");
        let plugins = root.join("plugins");
        let grants_path = config.join("permissions.toml");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(&plugins).unwrap();
        write_manifest(&plugins, runtime, scenario.behavior);
        write_grants(&grants_path, scenario.grants);
        let catalog = load_catalog(&plugins);
        assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
        let state = load_permission_state(&catalog, &grants_path);
        assert!(state.file_error.is_none(), "{:?}", state.file_error);
        let registry = CapabilityRegistry::default();
        registry.replace(state.grants);
        let (event_tx, events) = mpsc::channel();
        let (wake, wake_rx) = mpsc::channel();
        let environment = HostEnvironment::from_paths(Some(root.clone()), Some(config));
        let host = Arc::new(PluginHost::start(
            &catalog,
            registry.clone(),
            environment,
            event_tx,
            wake,
        ));
        let (finished_tx, finished) = mpsc::channel();
        let observation = Arc::new(Mutex::new(Observation::default()));
        let policy = Arc::new(Mutex::new(Policy::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let driver = Driver {
            host: Arc::clone(&host),
            registry: registry.clone(),
            observation: Arc::clone(&observation),
            policy: Arc::clone(&policy),
            stop: Arc::clone(&stop),
            finished: finished_tx,
        };
        let driver = driver.spawn(events);
        Self {
            root,
            grants_path,
            catalog,
            host,
            driver: Some(driver),
            stop,
            finished,
            observation,
            registry,
            policy,
            _wake: wake_rx,
        }
    }

    pub(super) fn activate_ready(&self) {
        self.host
            .activate(PLUGIN_NAME)
            .expect("activate conformance plugin");
        self.wait_ready();
    }

    pub(super) fn wait_ready(&self) {
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            match self.host.status(PLUGIN_NAME) {
                Some(PluginStatus::Ready) => return,
                Some(PluginStatus::Disabled) | Some(PluginStatus::Error(_)) => {
                    panic!("plugin did not start: {}", self.describe())
                }
                _ if Instant::now() >= deadline => {
                    panic!("plugin did not become ready: {}", self.describe())
                }
                _ => thread::sleep(STATUS_POLL),
            }
        }
    }

    pub(super) fn revoke_all(&self) {
        write_grants(&self.grants_path, &[]);
        let state = load_permission_state(&self.catalog, &self.grants_path);
        assert!(state.file_error.is_none(), "{:?}", state.file_error);
        self.registry.replace(state.grants);
    }

    pub(super) fn invoke(&self, command: &str, args: Value) -> Result<Value, PluginError> {
        let request = self
            .host
            .invoke(PLUGIN_NAME, command, args, context())
            .expect("invoke conformance plugin");
        self.wait_finished(request)
    }

    pub(super) fn notify(&self, event: &str, payload: Value) -> NotifyOutcome {
        self.host
            .notify(PLUGIN_NAME, event, payload)
            .expect("notify conformance plugin")
    }

    pub(super) fn set_stale_revision(&self, stale: bool) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .stale_revision = stale;
    }

    pub(super) fn wait_for_host_response(&self, method: &str) -> bool {
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            if self
                .observation()
                .host_requests
                .iter()
                .any(|request| request.method == method && request.responded)
            {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(STATUS_POLL);
        }
    }

    pub(super) fn observation(&self) -> Observation {
        self.observation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub(super) fn release_host_responses(&self) {
        let policy = self
            .policy
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let (released, condvar) = &*policy.release;
        *released.lock().unwrap_or_else(|error| error.into_inner()) = true;
        condvar.notify_all();
    }

    fn wait_finished(&self, request: u64) -> Result<Value, PluginError> {
        let deadline = Instant::now() + INVOCATION_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "invocation {request} timed out");
            match self.finished.recv_timeout(remaining) {
                Ok(finished) if finished.request == request => return finished.result,
                Ok(_) => {}
                Err(error) => panic!("invocation {request} did not finish: {error}"),
            }
        }
    }

    fn describe(&self) -> String {
        let observation = self.observation();
        format!(
            "status {:?}, logs {:?}",
            self.host.status(PLUGIN_NAME),
            observation.logs
        )
    }
}

impl Drop for ConformanceHost {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.release_host_responses();
        self.host.shutdown();
        if let Some(driver) = self.driver.take() {
            let _ = driver.join();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct Driver {
    host: Arc<PluginHost>,
    registry: CapabilityRegistry,
    observation: Arc<Mutex<Observation>>,
    policy: Arc<Mutex<Policy>>,
    stop: Arc<AtomicBool>,
    finished: Sender<Finished>,
}

impl Driver {
    fn spawn(self, events: Receiver<PluginHostEvent>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("kawi-conformance-driver".to_string())
            .spawn(move || self.run(events))
            .expect("spawn conformance driver")
    }

    fn run(self, events: Receiver<PluginHostEvent>) {
        while !self.stop.load(Ordering::SeqCst) {
            let event = match events.recv_timeout(DRIVER_POLL) {
                Ok(event) => event,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            self.handle(event);
        }
    }

    fn handle(&self, event: PluginHostEvent) {
        match event {
            PluginHostEvent::HostRequest {
                plugin,
                id,
                method,
                params,
            } => self.handle_host_request(&plugin, id, &method, params),
            PluginHostEvent::InvocationFinished {
                request, result, ..
            } => {
                let _ = self.finished.send(Finished { request, result });
            }
            PluginHostEvent::Log { line, .. } => self.observe_log(line),
            PluginHostEvent::StatusChanged { status, .. } => self.observe_status(status),
            PluginHostEvent::CapabilityRequired { capability, .. } => {
                self.observation
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .required
                    .push(capability);
            }
            PluginHostEvent::SandboxApplied { .. } => {}
        }
    }

    fn handle_host_request(&self, plugin: &str, id: Value, method: &str, params: Value) {
        let allowed = is_host_method_allowed(&self.registry, plugin, method);
        let index = self.record_host_request(method, !allowed);
        if !allowed {
            let _ = self
                .host
                .respond(plugin, id, Err(capability_denied(method)));
            self.mark_responded(index);
            return;
        }
        let (withhold, stale_revision, release) = self.policy_snapshot();
        if withhold && !wait_for_release(&release, &self.stop) {
            return;
        }
        let _ = self
            .host
            .respond(plugin, id, host_result(method, &params, stale_revision));
        self.mark_responded(index);
    }

    fn record_host_request(&self, method: &str, denied: bool) -> usize {
        let mut observation = self
            .observation
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        observation.host_requests.push(HostRequestRecord {
            method: method.to_string(),
            denied,
            responded: false,
        });
        observation.host_requests.len() - 1
    }

    fn mark_responded(&self, index: usize) {
        if let Some(request) = self
            .observation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .host_requests
            .get_mut(index)
        {
            request.responded = true;
        }
    }

    fn policy_snapshot(&self) -> (bool, bool, Arc<(Mutex<bool>, Condvar)>) {
        let policy = self
            .policy
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        (
            policy.withhold,
            policy.stale_revision,
            Arc::clone(&policy.release),
        )
    }

    fn observe_log(&self, line: String) {
        self.observation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .logs
            .push(line);
    }

    fn observe_status(&self, status: PluginStatus) {
        self.observation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .statuses
            .push(status);
    }
}

fn capability_denied(method: &str) -> RpcError {
    match host_method_capability(method) {
        MethodCapability::Required(capability) => {
            RpcError::capability_denied(capability.name(), None)
        }
        _ => RpcError::capability_denied("unknown", None),
    }
}

fn wait_for_release(release: &(Mutex<bool>, Condvar), stop: &AtomicBool) -> bool {
    let (released, condvar) = release;
    let mut released = released.lock().unwrap_or_else(|error| error.into_inner());
    while !*released {
        if stop.load(Ordering::SeqCst) {
            return false;
        }
        let (next, _) = condvar
            .wait_timeout(released, RELEASE_POLL)
            .unwrap_or_else(|error| error.into_inner());
        released = next;
    }
    true
}

fn host_result(method: &str, params: &Value, stale_revision: bool) -> Result<Value, RpcError> {
    match method {
        "editor.get_state" => Ok(state_value()),
        "editor.apply_edit" if stale_revision => Err(RpcError::stale_revision(
            params
                .get("revision")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
            7,
        )),
        "editor.apply_edit" => Ok(json!({ "revision": next_revision(params) })),
        "editor.set_diagnostics" | "ui.message" => Ok(Value::Null),
        _ => Ok(Value::Null),
    }
}

fn next_revision(params: &Value) -> u64 {
    params
        .get("revision")
        .and_then(Value::as_u64)
        .unwrap_or_default()
        .saturating_add(1)
}

fn state_value() -> Value {
    json!({
        "buffer": 1,
        "revision": 7,
        "path": "/tmp/conformance.rs",
        "language": "rust",
        "mode": "normal",
        "cursor": { "line": 0, "byte_column": 0 },
        "selection": null,
        "text": null,
    })
}

fn context() -> InvocationContext {
    InvocationContext {
        buffer: 1,
        revision: 7,
        path: Some("/tmp/conformance.rs".to_string()),
        language: Some("rust".to_string()),
        mode: "normal".to_string(),
        cursor: Position {
            line: 0,
            byte_column: 0,
        },
        selection: None,
    }
}

fn temp_root() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("kawi-conformance-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_grants(path: &Path, grants: &[Capability]) {
    let mut text = String::new();
    if !grants.is_empty() {
        text.push_str(&format!("[plugins.{PLUGIN_NAME}.grant]\n"));
        for capability in grants {
            text.push_str(&format!("\"{}\" = true\n", capability.name()));
        }
    }
    fs::write(path, text).unwrap();
}

fn write_manifest(plugins: &Path, runtime: Runtime, behavior: Behavior) {
    let dir = plugins.join(PLUGIN_NAME);
    fs::create_dir_all(&dir).unwrap();
    let capabilities = capability_table(behavior);
    let subscribes = subscribes_line(behavior);
    let manifest = match runtime {
        Runtime::Process => {
            let executable = env!("CARGO_BIN_EXE_fake-plugin");
            format!(
                "name = \"{PLUGIN_NAME}\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [{executable:?}, {:?}]\ntimeout_ms = 2000\n{subscribes}\n{capabilities}\n[[commands]]\nid = \"probe\"\ntitle = \"Probe\"\n",
                behavior.process_mode()
            )
        }
        Runtime::Wasm => {
            let source = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/wasm")
                .join(behavior.wasm_component());
            let bytes = fs::read(source).unwrap();
            fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
            let digest = Sha256::digest(&bytes);
            format!(
                "name = \"{PLUGIN_NAME}\"\nversion = \"0.1.0\"\nprotocol = 1\nruntime = \"wasm\"\ncomponent = \"plugin.wasm\"\nsha256 = \"sha256:{digest:x}\"\ntimeout_ms = 2000\n{subscribes}\n{capabilities}"
            )
        }
    };
    fs::write(dir.join("plugin.toml"), manifest).unwrap();
}

fn subscribes_line(behavior: Behavior) -> String {
    match behavior.event() {
        Some(event) => format!("subscribes = [\"{event}\"]\n"),
        None => String::new(),
    }
}

fn capability_table(behavior: Behavior) -> String {
    let requests = behavior.requests();
    if requests.is_empty() {
        return String::new();
    }
    let mut table = String::from("[capabilities]\n");
    for capability in requests {
        table.push_str(&format!("\"{}\" = true\n", capability.name()));
    }
    table
}
