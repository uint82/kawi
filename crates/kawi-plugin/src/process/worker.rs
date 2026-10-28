use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;

use super::backoff::Backoff;
use super::connection::{Connection, ConnectionConfig, WriterSlot};
use super::WorkerCommand;
use crate::capability::{Capability, CapabilityRegistry};
use crate::environment::HostEnvironment;
use crate::error::PluginError;
use crate::host::{send_event, set_status, PluginHostEvent, PluginStatus};
use crate::manifest::LoadedPlugin;
use crate::protocol::{InvocationContext, InvokeParams};

pub(super) struct InvocationJob {
    pub(super) request: u64,
    pub(super) command: String,
    pub(super) args: Value,
    pub(super) context: Box<InvocationContext>,
}

pub(super) struct WorkerContext {
    pub(super) events: Sender<PluginHostEvent>,
    pub(super) wake: Sender<()>,
    pub(super) writer: WriterSlot,
    pub(super) status: Arc<Mutex<PluginStatus>>,
    pub(super) event_filter: Arc<Mutex<BTreeSet<String>>>,
    pub(super) capabilities: CapabilityRegistry,
    pub(super) environment: HostEnvironment,
    pub(super) restart: Arc<AtomicBool>,
}

pub(super) struct Worker {
    plugin: LoadedPlugin,
    events: Sender<PluginHostEvent>,
    wake: Sender<()>,
    writer: WriterSlot,
    status: Arc<Mutex<PluginStatus>>,
    event_filter: Arc<Mutex<BTreeSet<String>>>,
    capabilities: CapabilityRegistry,
    environment: HostEnvironment,
    restart: Arc<AtomicBool>,
    timeout: Duration,
    connection: Option<Connection>,
    backoff: Backoff,
}

impl Worker {
    pub(super) fn new(plugin: LoadedPlugin, context: WorkerContext) -> Self {
        let timeout = Duration::from_millis(plugin.manifest.timeout_ms);
        Self {
            plugin,
            events: context.events,
            wake: context.wake,
            writer: context.writer,
            status: context.status,
            event_filter: context.event_filter,
            capabilities: context.capabilities,
            environment: context.environment,
            restart: context.restart,
            timeout,
            connection: None,
            backoff: Backoff::new(),
        }
    }

    pub(super) fn run(mut self, commands: std::sync::mpsc::Receiver<WorkerCommand>) {
        while let Ok(command) = commands.recv() {
            match command {
                WorkerCommand::Activate => self.activate(),
                WorkerCommand::Invoke {
                    request,
                    command,
                    args,
                    context,
                } => self.handle_invocation(InvocationJob {
                    request,
                    command,
                    args,
                    context,
                }),
                WorkerCommand::Restart => self.restart(),
                WorkerCommand::Shutdown => break,
            }
        }
        self.shutdown();
    }

    fn activate(&mut self) {
        if self.connection.is_some() {
            return;
        }
        if let Err(error) = self.start_connection() {
            self.backoff.record_failure();
            self.update_status(&Err(error.clone()));
            send_event(
                &self.events,
                &self.wake,
                PluginHostEvent::Log {
                    plugin: self.plugin.manifest.name.clone(),
                    line: error.to_string(),
                },
            );
        }
    }

    fn handle_invocation(&mut self, job: InvocationJob) {
        let request = job.request;
        let result = self.invoke(job);
        if matches!(&result, Err(error) if is_transport_failure(error)) {
            if let Some(mut failed) = self.connection.take() {
                failed.shutdown();
            }
        }
        self.update_backoff(&result);
        self.update_status(&result);
        if let Err(error) = &result {
            if is_transport_failure(error) {
                send_event(
                    &self.events,
                    &self.wake,
                    PluginHostEvent::Log {
                        plugin: self.plugin.manifest.name.clone(),
                        line: error.to_string(),
                    },
                );
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

    fn invoke(&mut self, job: InvocationJob) -> Result<Value, PluginError> {
        self.backoff.check()?;
        self.ensure_alive();
        if self.connection.is_none() {
            self.start_connection()?;
        }
        let context = if self
            .capabilities
            .has(&self.plugin.manifest.name, Capability::EditorRead)
        {
            *job.context
        } else {
            InvocationContext::redacted()
        };
        self.connection
            .as_mut()
            .ok_or(PluginError::ProcessExited)?
            .invoke(
                job.request,
                InvokeParams {
                    command: job.command,
                    args: job.args,
                    context,
                },
            )
    }

    fn start_connection(&mut self) -> Result<(), PluginError> {
        self.set_status(PluginStatus::Starting);
        let mut connection = Connection::start(
            &self.plugin,
            &self.events,
            &self.wake,
            Arc::clone(&self.writer),
            ConnectionConfig {
                timeout: self.timeout,
                event_filter: Arc::clone(&self.event_filter),
                capabilities: self.capabilities.clone(),
                environment: self.environment.clone(),
                restart: Arc::clone(&self.restart),
            },
        )?;
        if connection.is_sandboxed() {
            send_event(
                &self.events,
                &self.wake,
                PluginHostEvent::SandboxApplied {
                    plugin: self.plugin.manifest.name.clone(),
                },
            );
        }
        if let Err(error) = connection.initialize() {
            connection.shutdown();
            return Err(error);
        }
        self.connection = Some(connection);
        self.set_status(PluginStatus::Ready);
        Ok(())
    }

    fn ensure_alive(&mut self) {
        if self
            .connection
            .as_ref()
            .is_some_and(|current| !current.is_alive())
        {
            if let Some(mut dead) = self.connection.take() {
                dead.shutdown();
            }
        }
    }

    fn restart(&mut self) {
        if let Some(mut current) = self.connection.take() {
            current.shutdown();
        }
        self.restart.store(false, Ordering::SeqCst);
        self.backoff.reset();
        self.set_status(PluginStatus::Stopped);
    }

    fn shutdown(&mut self) {
        if let Some(mut current) = self.connection.take() {
            current.shutdown();
        }
        self.set_status(PluginStatus::Stopped);
    }

    fn update_backoff(&mut self, result: &Result<Value, PluginError>) {
        if matches!(result, Err(error) if is_transport_failure(error)) {
            self.backoff.record_failure();
        } else if result.is_ok() || matches!(result, Err(PluginError::Rpc(_))) {
            self.backoff.record_success();
        }
    }

    fn update_status(&self, result: &Result<Value, PluginError>) {
        if matches!(result, Err(error) if is_transport_failure(error)) {
            if self.backoff.is_disabled() {
                self.set_status(PluginStatus::Disabled);
            } else {
                self.set_status(PluginStatus::Backoff {
                    remaining_ms: self.backoff.remaining().as_millis() as u64,
                });
            }
        } else if result.is_ok() || matches!(result, Err(PluginError::Rpc(_))) {
            self.set_status(PluginStatus::Ready);
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
}

fn is_transport_failure(error: &PluginError) -> bool {
    matches!(
        error,
        PluginError::Spawn(_)
            | PluginError::Timeout
            | PluginError::ProcessExited
            | PluginError::Protocol(_)
            | PluginError::Initialize(_)
    )
}
