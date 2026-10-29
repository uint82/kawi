use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
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
    invalid_position: bool,
    withhold: bool,
    deferred: Vec<DeferredResponse>,
}

struct DeferredResponse {
    plugin: String,
    id: Value,
    result: Result<Value, RpcError>,
    index: usize,
}

impl Policy {
    fn new() -> Self {
        Self {
            stale_revision: false,
            invalid_position: false,
            withhold: false,
            deferred: Vec::new(),
        }
    }
}

pub(super) struct ConformanceHost {
    runtime: Runtime,
    scenario: &'static str,
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
        if !scenario.behavior.expects_load_warning() {
            assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
        }
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
            runtime,
            scenario: scenario.id,
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
                    panic!(
                        "{}: plugin did not start: {}",
                        self.label(),
                        self.describe()
                    )
                }
                _ if Instant::now() >= deadline => {
                    panic!(
                        "{}: plugin did not become ready: {}",
                        self.label(),
                        self.describe()
                    )
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
        match self.start_invocation(command, args) {
            Ok(request) => self.wait_for_finished(request),
            Err(error) => Err(error),
        }
    }

    pub(super) fn start_invocation(&self, command: &str, args: Value) -> Result<u64, PluginError> {
        self.host.invoke(PLUGIN_NAME, command, args, context())
    }

    pub(super) fn wait_for_finished(&self, request: u64) -> Result<Value, PluginError> {
        let deadline = Instant::now() + INVOCATION_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "{}: invocation {request} timed out",
                self.label()
            );
            match self.finished.recv_timeout(remaining) {
                Ok(finished) if finished.request == request => return finished.result,
                Ok(_) => {}
                Err(error) => panic!(
                    "{}: invocation {request} did not finish: {error}",
                    self.label()
                ),
            }
        }
    }

    pub(super) fn restart(&self) {
        self.host
            .restart(PLUGIN_NAME)
            .expect("restart conformance plugin");
    }

    pub(super) fn status(&self) -> Option<PluginStatus> {
        self.host.status(PLUGIN_NAME)
    }

    pub(super) fn wait_backoff(&self) {
        if let Some(PluginStatus::Backoff { remaining_ms }) = self.status() {
            thread::sleep(Duration::from_millis(remaining_ms.saturating_add(10)));
        }
    }

    pub(super) fn wait_for_host_request(&self, method: &str) -> bool {
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            if self
                .observation()
                .host_requests
                .iter()
                .any(|request| request.method == method)
            {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(STATUS_POLL);
        }
    }

    pub(super) fn notify(&self, event: &str, payload: Value) -> NotifyOutcome {
        self.host
            .notify(PLUGIN_NAME, event, payload)
            .expect("notify conformance plugin")
    }

    pub(super) fn stall_events(&self, event: &str, payload: Value) {
        match self.runtime {
            Runtime::Process => self.inject_pipe_block(),
            Runtime::Wasm => {
                self.withhold_host_responses();
                assert_eq!(
                    self.notify(event, payload),
                    NotifyOutcome::Queued,
                    "{}: {event} was not queued",
                    self.label()
                );
                assert!(
                    self.wait_for_host_request("ui.message"),
                    "{}: {event} was not delivered before revocation",
                    self.label()
                );
            }
        }
    }

    pub(super) fn release_events(&self) {
        if self.runtime == Runtime::Wasm {
            self.release_host_responses();
        }
    }

    pub(super) fn host_request_count(&self) -> usize {
        self.observation().host_requests.len()
    }

    fn inject_pipe_block(&self) {
        self.host
            .respond(
                PLUGIN_NAME,
                json!(999),
                Ok(json!({ "pad": "x".repeat(1024 * 1024) })),
            )
            .expect("inject pipe block");
    }

    pub(super) fn set_stale_revision(&self, stale: bool) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .stale_revision = stale;
    }

    pub(super) fn set_invalid_position(&self, invalid: bool) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .invalid_position = invalid;
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

    pub(super) fn withhold_host_responses(&self) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .withhold = true;
    }

    pub(super) fn release_host_responses(&self) {
        let deferred = {
            let mut policy = self
                .policy
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            policy.withhold = false;
            std::mem::take(&mut policy.deferred)
        };
        for response in deferred {
            let _ = self
                .host
                .respond(&response.plugin, response.id, response.result);
            self.mark_responded(response.index);
        }
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

    fn label(&self) -> String {
        format!("{:?} {}", self.runtime, self.scenario)
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
        let (withhold, stale_revision, invalid_position) = self.policy_snapshot();
        let result = host_result(method, &params, stale_revision, invalid_position);
        if withhold {
            self.defer_response(plugin, id, result, index);
            return;
        }
        let _ = self.host.respond(plugin, id, result);
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

    fn policy_snapshot(&self) -> (bool, bool, bool) {
        let policy = self
            .policy
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        (
            policy.withhold,
            policy.stale_revision,
            policy.invalid_position,
        )
    }

    fn defer_response(
        &self,
        plugin: &str,
        id: Value,
        result: Result<Value, RpcError>,
        index: usize,
    ) {
        self.policy
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .deferred
            .push(DeferredResponse {
                plugin: plugin.to_string(),
                id,
                result,
                index,
            });
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

fn host_result(
    method: &str,
    params: &Value,
    stale_revision: bool,
    invalid_position: bool,
) -> Result<Value, RpcError> {
    match method {
        "editor.get_state" => Ok(state_value()),
        "editor.apply_edit" if invalid_position => {
            Err(RpcError::invalid_position("edit position is out of range"))
        }
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
            let bytes = if behavior.oversizes_component() {
                pad_component(bytes)
            } else {
                bytes
            };
            fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
            let digest = Sha256::digest(&bytes);
            let hash = if behavior == Behavior::HashMismatch {
                "0".repeat(64)
            } else {
                format!("{digest:x}")
            };
            format!(
                "name = \"{PLUGIN_NAME}\"\nversion = \"0.1.0\"\nprotocol = 1\nruntime = \"wasm\"\ncomponent = \"plugin.wasm\"\nsha256 = \"sha256:{hash}\"\ntimeout_ms = 2000\n{subscribes}\n{capabilities}"
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

fn pad_component(mut bytes: Vec<u8>) -> Vec<u8> {
    const MAX_COMPONENT_BYTES: usize = 64 * 1024 * 1024;
    const PADDING_NAME: &[u8] = b"conformance-padding";
    let payload_len = MAX_COMPONENT_BYTES
        .saturating_sub(bytes.len())
        .saturating_add(1);
    let mut section = Vec::new();
    write_leb128(PADDING_NAME.len(), &mut section);
    section.extend_from_slice(PADDING_NAME);
    section.resize(section.len() + payload_len, 0);
    bytes.push(0);
    write_leb128(section.len(), &mut bytes);
    bytes.extend_from_slice(&section);
    bytes
}

fn write_leb128(mut value: usize, output: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            break;
        }
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
