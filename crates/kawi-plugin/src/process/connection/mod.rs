mod channel;
mod interaction;
mod read;
mod startup;
mod write;

use std::collections::{BTreeSet, HashMap};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

use self::channel::{fail_pending, remove_pending, PendingMap, ReaderContext, WriterContext};
pub(crate) use self::interaction::Interaction;
use self::read::spawn_reader;
use self::startup::{spawn_error_message, validate_initialize};
use self::write::{spawn_stderr, spawn_writer};
use crate::capability::{advertised_events, advertised_host_methods, CapabilityRegistry};
use crate::environment::HostEnvironment;
use crate::error::PluginError;
use crate::host::{send_event, PluginHostEvent};
use crate::manifest::LoadedPlugin;
use crate::protocol::{
    notification_value, request_value, HostInfo, InitializeParams, InitializeResult, InvokeParams,
    PLUGIN_METHODS, POSITION_ENCODING, PROTOCOL_MINOR, PROTOCOL_VERSION,
};

pub(super) type WriterSlot = Arc<Mutex<Option<SyncSender<Value>>>>;

const INITIALIZE_REQUEST_ID: u64 = 0;
const SUPERVISOR_QUEUE_CAPACITY: usize = 1024;
const RESTART_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(super) struct ConnectionConfig {
    pub(super) timeout: Duration,
    pub(super) event_filter: Arc<Mutex<BTreeSet<String>>>,
    pub(super) capabilities: CapabilityRegistry,
    pub(super) environment: HostEnvironment,
    pub(super) restart: Arc<AtomicBool>,
    pub(super) interaction: Arc<Interaction>,
}

pub(super) struct Connection {
    child: Child,
    writer: SyncSender<Value>,
    pending: PendingMap,
    alive: Arc<AtomicBool>,
    event_filter: Arc<Mutex<BTreeSet<String>>>,
    subscribes: BTreeSet<String>,
    capabilities: CapabilityRegistry,
    plugin_name: String,
    writer_slot: WriterSlot,
    timeout: Duration,
    sandboxed: kawi_process::SandboxStatus,
    restart: Arc<AtomicBool>,
    interaction: Arc<Interaction>,
}

impl Connection {
    pub(super) fn start(
        plugin: &LoadedPlugin,
        events: &Sender<PluginHostEvent>,
        wake: &Sender<()>,
        writer_slot: WriterSlot,
        config: ConnectionConfig,
    ) -> Result<Self, PluginError> {
        let ConnectionConfig {
            timeout,
            event_filter,
            capabilities,
            environment,
            restart,
            interaction,
        } = config;
        let command = plugin.resolved_command();
        let executable = command
            .first()
            .ok_or_else(|| PluginError::Spawn("empty command".to_string()))?;
        let mut process = Command::new(executable);
        process
            .args(&command[1..])
            .current_dir(&plugin.dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        kawi_process::configure(&mut process);
        let plan = super::sandbox::policy(plugin, &capabilities, &environment);
        for warning in &plan.warnings {
            send_event(
                events,
                wake,
                PluginHostEvent::Log {
                    plugin: plugin.manifest.name.clone(),
                    line: warning.clone(),
                },
            );
        }
        let sandboxed = kawi_process::configure_sandbox(&mut process, &plan.policy);
        if sandboxed == kawi_process::SandboxStatus::Partial {
            send_event(
                events,
                wake,
                PluginHostEvent::Log {
                    plugin: plugin.manifest.name.clone(),
                    line: "landlock supports only partial enforcement; filesystem scopes stay advisory"
                        .to_string(),
                },
            );
        }
        let mut child = process.spawn().map_err(|error| {
            PluginError::Spawn(spawn_error_message(
                &plugin.manifest.name,
                sandboxed,
                &error,
            ))
        })?;
        let Some(stdin) = child.stdin.take() else {
            kawi_process::terminate(&mut child);
            return Err(PluginError::Spawn("missing stdin".to_string()));
        };
        let Some(stdout) = child.stdout.take() else {
            kawi_process::terminate(&mut child);
            return Err(PluginError::Spawn("missing stdout".to_string()));
        };
        let Some(stderr) = child.stderr.take() else {
            kawi_process::terminate(&mut child);
            return Err(PluginError::Spawn("missing stderr".to_string()));
        };
        let alive = Arc::new(AtomicBool::new(true));
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (writer, writer_rx) = mpsc::sync_channel(SUPERVISOR_QUEUE_CAPACITY);
        let plugin_name = plugin.manifest.name.clone();
        let reader_context = Arc::new(ReaderContext {
            alive: Arc::clone(&alive),
            pending: Arc::clone(&pending),
            writer: writer.clone(),
            events: events.clone(),
            wake: wake.clone(),
            plugin: plugin_name.clone(),
            interaction: Arc::clone(&interaction),
        });
        let writer_context = Arc::new(WriterContext {
            alive: Arc::clone(&alive),
            pending: Arc::clone(&pending),
            events: events.clone(),
            wake: wake.clone(),
            plugin: plugin_name.clone(),
            capabilities: capabilities.clone(),
        });
        if let Err(error) = spawn_writer(stdin, writer_rx, writer_context) {
            kawi_process::terminate(&mut child);
            return Err(error);
        }
        if let Err(error) = spawn_reader(stdout, reader_context) {
            kawi_process::terminate(&mut child);
            return Err(error);
        }
        if let Err(error) = spawn_stderr(stderr, events.clone(), wake.clone(), plugin_name) {
            kawi_process::terminate(&mut child);
            return Err(error);
        }
        *writer_slot
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(writer.clone());
        Ok(Self {
            child,
            writer,
            pending,
            alive,
            event_filter,
            subscribes: plugin.manifest.subscribes.iter().cloned().collect(),
            capabilities,
            plugin_name: plugin.manifest.name.clone(),
            writer_slot,
            timeout,
            sandboxed,
            restart,
            interaction,
        })
    }

    pub(super) fn initialize(&mut self) -> Result<InitializeResult, PluginError> {
        let params = InitializeParams {
            protocol_version: PROTOCOL_VERSION,
            protocol_minor: PROTOCOL_MINOR,
            host: HostInfo {
                name: "kawi".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            position_encoding: POSITION_ENCODING.to_string(),
            host_capabilities: advertised_host_methods(&self.capabilities, &self.plugin_name)
                .into_iter()
                .map(str::to_string)
                .collect(),
            plugin_capabilities: PLUGIN_METHODS
                .iter()
                .map(|method| (*method).to_string())
                .collect(),
            events: advertised_events(&self.capabilities, &self.plugin_name)
                .into_iter()
                .map(str::to_string)
                .collect(),
        };
        let params = serde_json::to_value(params)
            .map_err(|error| PluginError::Initialize(error.to_string()))?;
        let value = self.request(INITIALIZE_REQUEST_ID, "initialize", params)?;
        let result: InitializeResult = serde_json::from_value(value)
            .map_err(|error| PluginError::Initialize(error.to_string()))?;
        validate_initialize(&result)?;
        let subscribed = self
            .subscribes
            .iter()
            .filter(|event| result.events.contains(event))
            .cloned()
            .collect();
        *self
            .event_filter
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = subscribed;
        Ok(result)
    }

    pub(super) fn invoke(
        &mut self,
        request: u64,
        params: InvokeParams,
    ) -> Result<Value, PluginError> {
        let params = serde_json::to_value(params)
            .map_err(|error| PluginError::Protocol(error.to_string()))?;
        self.request(request, "invoke_command", params)
    }

    pub(super) fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    pub(super) fn is_sandboxed(&self) -> bool {
        self.sandboxed.is_enforced()
    }

    pub(super) fn shutdown(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        self.interaction.reset();
        *self
            .writer_slot
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
        fail_pending(&self.pending, PluginError::Shutdown);
        kawi_process::terminate(&mut self.child);
    }

    fn request(&mut self, id: u64, method: &str, params: Value) -> Result<Value, PluginError> {
        let (response_tx, response_rx) = mpsc::channel();
        self.pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(id, response_tx);
        if self
            .writer
            .try_send(request_value(Value::from(id), method, params))
            .is_err()
        {
            remove_pending(&self.pending, id);
            return Err(PluginError::QueueFull);
        }
        let cancellable = method != "initialize";
        let started = Instant::now();
        loop {
            let remaining = self.interaction.remaining(started, self.timeout);
            if remaining.is_zero() {
                return self.cancel_request(id, PluginError::Timeout);
            }
            match response_rx.recv_timeout(remaining.min(RESTART_POLL_INTERVAL)) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if cancellable && self.restart.load(Ordering::SeqCst) {
                        return self.cancel_request(id, PluginError::Restart);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return self.cancel_request(id, PluginError::ProcessExited);
                }
            }
        }
    }

    fn cancel_request(&self, id: u64, error: PluginError) -> Result<Value, PluginError> {
        remove_pending(&self.pending, id);
        let _ = self.writer.try_send(notification_value(
            "$/cancelRequest",
            serde_json::json!({"id": id}),
        ));
        Err(error)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.shutdown();
    }
}
