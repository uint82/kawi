use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::Value;

use crate::capability::CapabilityRegistry;
use crate::environment::HostEnvironment;
use crate::error::PluginError;
use crate::host::{send_event, set_status, PluginHostEvent, PluginStatus};
use crate::manifest::LoadedPlugin;
use crate::process::Backoff;
use crate::protocol::{InvocationContext, RpcError};

use self::handle::WasmCommand;
pub(crate) use self::handle::WasmWorkerHandle;
use super::session::Session;
use super::state::{pending_calls, PendingCalls};

const COMMAND_QUEUE: usize = 64;

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
    let handle = WasmWorkerHandle {
        commands,
        status: Arc::clone(&status),
        event_filter: Arc::clone(&event_filter),
        pending: Arc::clone(&pending),
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
    session: Option<Session>,
    start_error: Option<String>,
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
                self.backoff.record_success();
                self.set_status(PluginStatus::Ready);
            }
            Err(error) => {
                self.start_error = Some(error.to_string());
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
        if let Err(error) = self.backoff.check() {
            send_event(
                &self.events,
                &self.wake,
                PluginHostEvent::InvocationFinished {
                    plugin: self.plugin.manifest.name.clone(),
                    request,
                    result: Err(error),
                },
            );
            self.update_backoff_status();
            return;
        }
        self.ensure_start();
        let had_session = self.session.is_some();
        let not_ready = || {
            PluginError::Initialize(
                self.start_error
                    .clone()
                    .unwrap_or_else(|| "wasm session is not ready".to_string()),
            )
        };
        let result = self
            .session
            .as_mut()
            .ok_or_else(not_ready)
            .and_then(|session| session.invoke(&command, args));
        match &result {
            Ok(_) => self.backoff.record_success(),
            Err(error) => {
                self.session = None;
                if had_session {
                    self.backoff.record_failure();
                    self.update_backoff_status();
                }
                self.log(error.to_string());
            }
        }
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

    fn handle_event(&mut self, event: &str, payload: Value) {
        if let Err(error) = self.backoff.check() {
            self.log(error.to_string());
            return;
        }
        self.ensure_start();
        let had_session = self.session.is_some();
        let result = self
            .session
            .as_mut()
            .ok_or(PluginError::ProcessExited)
            .and_then(|session| session.event(event, payload));
        if let Err(error) = result {
            self.log(error.to_string());
            self.session = None;
            if had_session {
                self.backoff.record_failure();
                self.update_backoff_status();
            }
        }
    }

    fn restart(&mut self) {
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
        fail_pending(&self.pending);
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

fn fail_pending(pending: &PendingCalls) {
    let mut pending = pending.lock().unwrap_or_else(|error| error.into_inner());
    let calls = std::mem::take(&mut *pending);
    for (_, sender) in calls {
        let _ = sender.send(Err(RpcError::unavailable("plugin stopped")));
    }
}

mod handle;
#[cfg(test)]
mod tests;
