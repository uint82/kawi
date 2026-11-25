use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::Value;

use super::backoff::Backoff;
use crate::capability::{is_event_allowed, CapabilityRegistry};
use crate::environment::HostEnvironment;
use crate::error::PluginError;
use crate::host::{send_event, set_status, PluginHostEvent, PluginStatus};
use crate::manifest::LoadedPlugin;
use crate::protocol::{InvocationContext, RpcError};

use self::handle::WasmCommand;
pub(crate) use self::handle::WasmWorkerHandle;
use super::session::Session;
use super::state::{fail_pending, pending_calls, PendingCalls};

const COMMAND_QUEUE: usize = 64;
const SESSION_NOT_READY: &str = "wasm session is not ready";

pub(crate) fn start_wasm_worker(
    plugin: LoadedPlugin,
    capabilities: CapabilityRegistry,
    environment: HostEnvironment,
    events: mpsc::Sender<PluginHostEvent>,
    wake: mpsc::Sender<()>,
) -> WasmWorkerHandle {
    let (commands, receiver) = mpsc::sync_channel(COMMAND_QUEUE);
    let status = Arc::new(Mutex::new(PluginStatus::Stopped));
    let event_filter = Arc::new(Mutex::new(
        plugin.manifest.subscribes.iter().cloned().collect(),
    ));
    let pending = pending_calls();
    let cancel = Arc::new(AtomicBool::new(false));
    let handle = WasmWorkerHandle {
        commands,
        status: Arc::clone(&status),
        event_filter: Arc::clone(&event_filter),
        pending: Arc::clone(&pending),
        cancel: Arc::clone(&cancel),
        requirements: Arc::new(plugin.command_requirements()),
    };
    let name = plugin.manifest.name.clone();
    let worker = WasmWorker {
        plugin,
        capabilities,
        environment,
        events: events.clone(),
        wake: wake.clone(),
        status,
        pending,
        cancel,
        session: None,
        start_error: None,
        backoff: Backoff::new(),
    };
    let error_status = Arc::clone(&handle.status);
    let error_name = name.clone();
    let spawn = thread::Builder::new()
        .name(format!("kawi-wasm-{name}"))
        .spawn(move || worker.run(receiver));
    if let Err(error) = spawn {
        set_status(
            &error_status,
            &events,
            &wake,
            &error_name,
            PluginStatus::Error(error.to_string()),
        );
    }
    handle
}

struct WasmWorker {
    plugin: LoadedPlugin,
    capabilities: CapabilityRegistry,
    environment: HostEnvironment,
    events: mpsc::Sender<PluginHostEvent>,
    wake: mpsc::Sender<()>,
    status: Arc<Mutex<PluginStatus>>,
    pending: PendingCalls,
    cancel: Arc<AtomicBool>,
    session: Option<Session>,
    start_error: Option<PluginError>,
    backoff: Backoff,
}

impl WasmWorker {
    fn run(mut self, commands: Receiver<WasmCommand>) {
        while let Ok(command) = commands.recv() {
            match command {
                WasmCommand::Activate => self.activate(),
                WasmCommand::Invoke {
                    request,
                    command,
                    args,
                    context,
                } => self.handle_invocation(request, command, args, context),
                WasmCommand::Notify { event, payload } => self.handle_event(&event, payload),
                WasmCommand::Callback {
                    registration,
                    event,
                    payload,
                } => self.handle_callback(registration, &event, payload),
                WasmCommand::TimerCallback { timer, payload } => {
                    self.handle_timer_callback(timer, payload)
                }
                WasmCommand::Restart => self.restart(),
                WasmCommand::Shutdown => break,
            }
        }
        self.shutdown();
    }

    fn start(&mut self) {
        if self.session.is_some() {
            return;
        }
        self.set_status(PluginStatus::Starting);
        match self.build_session().and_then(|mut session| {
            session.initialize()?;
            Ok(session)
        }) {
            Ok(session) => {
                self.session = Some(session);
                self.start_error = None;
                self.set_status(PluginStatus::Ready);
            }
            Err(error) => {
                self.emit_capability_requirements(&error);
                self.start_error = Some(error.clone());
                self.backoff.record_failure();
                self.update_backoff_status();
                self.log(error.to_string());
            }
        }
    }

    fn build_session(&self) -> Result<Session, PluginError> {
        Session::open(
            &self.plugin,
            &self.capabilities,
            &self.environment,
            self.events.clone(),
            self.wake.clone(),
            Arc::clone(&self.pending),
            Arc::clone(&self.cancel),
        )
    }

    fn activate(&mut self) {
        if let Err(error) = self.backoff.check() {
            self.log(error.to_string());
            self.update_backoff_status();
            return;
        }
        self.ensure_start();
    }

    fn ensure_start(&mut self) {
        if self.session.is_none() {
            self.start();
        }
    }

    fn handle_invocation(
        &mut self,
        request: u64,
        command: String,
        args: Value,
        _context: InvocationContext,
    ) {
        if let Some(capability) = self.plugin.denied_command(&self.capabilities, &command) {
            self.finish_request(
                request,
                Err(PluginError::Rpc(RpcError::capability_denied(
                    capability.name(),
                    None,
                ))),
            );
            return;
        }
        let result = match self.backoff.check() {
            Ok(()) => self.service_call(|session| session.invoke(&command, args)),
            Err(error) => {
                self.update_backoff_status();
                Err(error)
            }
        };
        self.finish_request(request, result);
    }

    fn finish_request(&self, request: u64, result: Result<Value, PluginError>) {
        send_event(
            &self.events,
            &self.wake,
            PluginHostEvent::InvocationFinished {
                plugin: self.plugin.manifest.name.clone(),
                request,
                result,
            },
        );
    }

    /// Runs one guest entry point and keeps the session only while the guest is
    /// healthy. RPC errors are valid replies, so they do not count as crashes.
    fn service_call<T>(
        &mut self,
        call: impl FnOnce(&mut Session) -> Result<T, PluginError>,
    ) -> Result<T, PluginError> {
        self.ensure_start();
        let Some(mut session) = self.session.take() else {
            return Err(self.session_unavailable());
        };
        let result = call(&mut session);
        if matches!(&result, Err(error) if error.is_transport_failure()) {
            self.backoff.record_failure();
            self.update_backoff_status();
        } else {
            self.session = Some(session);
            self.backoff.record_success();
            self.set_status(PluginStatus::Ready);
        }
        if let Err(error) = &result {
            self.log(error.to_string());
        }
        result
    }

    fn session_unavailable(&self) -> PluginError {
        self.start_error
            .clone()
            .unwrap_or_else(|| PluginError::Initialize(SESSION_NOT_READY.to_string()))
    }

    fn handle_event(&mut self, event: &str, payload: Value) {
        if !is_event_allowed(&self.capabilities, &self.plugin.manifest.name, event) {
            return;
        }
        if let Err(error) = self.backoff.check() {
            self.log(error.to_string());
            return;
        }
        let _ = self.service_call(|session| session.event(event, payload));
    }

    fn handle_timer_callback(&mut self, timer: u64, payload: Option<String>) {
        if let Err(error) = self.backoff.check() {
            self.log(error.to_string());
            return;
        }
        let _ = self.service_call(|session| session.timer_callback(timer, payload));
    }

    fn handle_callback(&mut self, registration: u64, event: &str, payload: Value) {
        if let Err(error) = self.backoff.check() {
            self.log(error.to_string());
            return;
        }
        let _ = self.service_call(|session| session.callback(registration, event, payload));
    }

    fn restart(&mut self) {
        self.cancel.store(false, Ordering::SeqCst);
        self.session = None;
        self.start_error = None;
        self.backoff.reset();
        self.set_status(PluginStatus::Stopped);
        self.start();
    }

    fn shutdown(&mut self) {
        self.session = None;
        self.start_error = None;
        self.set_status(PluginStatus::Stopped);
        fail_pending(
            &self.pending,
            RpcError::new(crate::protocol::PLUGIN_UNAVAILABLE, "plugin stopped"),
        );
    }

    fn update_backoff_status(&self) {
        if self.backoff.is_disabled() {
            self.set_status(PluginStatus::Disabled);
        } else {
            let remaining_ms = self.backoff.remaining().as_millis() as u64;
            self.set_status(PluginStatus::Backoff { remaining_ms });
        }
    }

    fn set_status(&self, status: PluginStatus) {
        set_status(
            &self.status,
            &self.events,
            &self.wake,
            &self.plugin.manifest.name,
            status,
        );
    }

    fn log(&self, line: String) {
        send_event(
            &self.events,
            &self.wake,
            PluginHostEvent::Log {
                plugin: self.plugin.manifest.name.clone(),
                line,
            },
        );
    }
}

mod consent;
mod handle;
#[cfg(test)]
mod tests;
