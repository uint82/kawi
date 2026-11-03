mod control;
mod events;
mod monitor;
mod open;
mod output;
mod stdin;
mod validate;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::process::Child;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::capability::CapabilityRegistry;
use crate::environment::HostEnvironment;
use crate::host::{send_event, PluginHostEvent};
use crate::manifest::LoadedPlugin;
use crate::protocol::{
    JobState, ProcessJobEvent, ProcessJobParams, ProcessOpenParams, ProcessOpenResult,
    ProcessSignal, ProcessSignalParams, ProcessStatusResult, ProcessWriteParams, RpcError,
    JOB_NOT_FOUND, MAX_JOBS_PER_PLUGIN, MAX_JOB_STDIN_BYTES,
};

use self::control::{ensure_running, send_signal, set_state, terminate};
use self::events::JobEventQueue;
use self::monitor::{spawn_monitor, JobStatus};
use self::output::{spawn_output, OutputBudget, OutputStream, OutputTracker};
use self::stdin::StdinHandle;

const JOB_RETENTION: Duration = Duration::from_secs(300);
const OUTPUT_STREAMS: usize = 2;

pub(crate) struct JobManager {
    jobs: Mutex<BTreeMap<u64, JobHandle>>,
    next_job: AtomicU64,
    capabilities: CapabilityRegistry,
    environment: HostEnvironment,
    events: std::sync::mpsc::Sender<PluginHostEvent>,
    wake: std::sync::mpsc::Sender<()>,
}

#[derive(Clone)]
struct JobHandle {
    plugin: String,
    child: Arc<Mutex<Child>>,
    stdin: StdinHandle,
    status: Arc<Mutex<JobStatus>>,
    output: Arc<OutputBudget>,
    events: Arc<Mutex<JobEventQueue>>,
    queue_events: bool,
}

impl JobManager {
    pub(crate) fn new(
        capabilities: CapabilityRegistry,
        environment: HostEnvironment,
        events: std::sync::mpsc::Sender<PluginHostEvent>,
        wake: std::sync::mpsc::Sender<()>,
    ) -> Self {
        Self {
            jobs: Mutex::new(BTreeMap::new()),
            next_job: AtomicU64::new(1),
            capabilities,
            environment,
            events,
            wake,
        }
    }

    pub(crate) fn open(
        &self,
        plugin: &LoadedPlugin,
        params: ProcessOpenParams,
    ) -> Result<ProcessOpenResult, RpcError> {
        self.prune();
        if self.running_jobs(&plugin.manifest.name) >= MAX_JOBS_PER_PLUGIN {
            return Err(RpcError::new(
                crate::protocol::JOB_LIMIT,
                "too many running jobs",
            ));
        }
        let (handle, stdout, stderr) =
            open::spawn(plugin, &self.capabilities, &self.environment, params)?;
        let job = self.next_job.fetch_add(1, Ordering::Relaxed);
        let tracker = OutputTracker::new(OUTPUT_STREAMS);
        lock(&self.jobs).insert(job, handle.clone());
        spawn_output(
            stdout,
            &self.events,
            &self.wake,
            OutputStream {
                handle: handle.clone(),
                job,
                stream: crate::protocol::JobStream::Stdout,
                budget: Arc::clone(&handle.output),
                tracker: Arc::clone(&tracker),
            },
        );
        spawn_output(
            stderr,
            &self.events,
            &self.wake,
            OutputStream {
                handle: handle.clone(),
                job,
                stream: crate::protocol::JobStream::Stderr,
                budget: Arc::clone(&handle.output),
                tracker: Arc::clone(&tracker),
            },
        );
        spawn_monitor(&self.events, &self.wake, job, &handle, tracker);
        Ok(ProcessOpenResult { job })
    }

    pub(crate) fn write(&self, plugin: &str, params: ProcessWriteParams) -> Result<(), RpcError> {
        let handle = self.entry(plugin, params.job)?;
        ensure_running(&handle)?;
        if params.data.len() > MAX_JOB_STDIN_BYTES {
            return Err(RpcError::invalid_params(
                "job input exceeds the maximum size",
            ));
        }
        handle.stdin.write(params.data.into_bytes())
    }

    pub(crate) fn close_stdin(
        &self,
        plugin: &str,
        params: ProcessJobParams,
    ) -> Result<(), RpcError> {
        let handle = self.entry(plugin, params.job)?;
        handle.stdin.close()
    }

    pub(crate) fn signal(&self, plugin: &str, params: ProcessSignalParams) -> Result<(), RpcError> {
        let handle = self.entry(plugin, params.job)?;
        let mut child = lock(&handle.child);
        ensure_running(&handle)?;
        send_signal(&mut child, params.signal)?;
        if params.signal == ProcessSignal::Kill {
            set_state(&handle.status, JobState::Killed);
        }
        Ok(())
    }

    pub(crate) fn kill(&self, plugin: &str, params: ProcessJobParams) -> Result<(), RpcError> {
        let handle = self.entry(plugin, params.job)?;
        terminate(&handle);
        Ok(())
    }

    pub(crate) fn status(
        &self,
        plugin: &str,
        params: ProcessJobParams,
    ) -> Result<ProcessStatusResult, RpcError> {
        let handle = self.entry(plugin, params.job)?;
        let status = lock(&handle.status);
        Ok(ProcessStatusResult {
            state: status.state,
            exit_code: status.exit_code,
            signal: status.signal,
        })
    }

    pub(crate) fn poll(
        &self,
        plugin: &str,
        params: ProcessJobParams,
    ) -> Result<Vec<ProcessJobEvent>, RpcError> {
        let handle = self.entry(plugin, params.job)?;
        let events = lock(&handle.events).drain();
        Ok(events)
    }

    pub(crate) fn stop_plugin(&self, plugin: &str) {
        let removed = {
            let mut jobs = lock(&self.jobs);
            let mut removed = Vec::new();
            jobs.retain(|_, handle| {
                if handle.plugin == plugin {
                    removed.push(handle.clone());
                    false
                } else {
                    true
                }
            });
            removed
        };
        for handle in removed {
            terminate(&handle);
        }
    }

    pub(crate) fn shutdown(&self) {
        let jobs = std::mem::take(&mut *lock(&self.jobs));
        for handle in jobs.into_values() {
            terminate(&handle);
        }
    }

    fn entry(&self, plugin: &str, job: u64) -> Result<JobHandle, RpcError> {
        lock(&self.jobs)
            .get(&job)
            .filter(|handle| handle.plugin == plugin)
            .cloned()
            .ok_or_else(|| RpcError::new(JOB_NOT_FOUND, "unknown job"))
    }

    fn running_jobs(&self, plugin: &str) -> usize {
        lock(&self.jobs)
            .values()
            .filter(|handle| handle.plugin == plugin && handle.state() == JobState::Running)
            .count()
    }

    fn prune(&self) {
        let now = Instant::now();
        lock(&self.jobs).retain(|_, handle| {
            let status = lock(&handle.status);
            status
                .exited_at
                .is_none_or(|at| now.duration_since(at) < JOB_RETENTION)
        });
    }
}

impl Drop for JobManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl JobHandle {
    fn state(&self) -> JobState {
        lock(&self.status).state
    }

    fn emit(
        &self,
        events: &std::sync::mpsc::Sender<PluginHostEvent>,
        wake: &std::sync::mpsc::Sender<()>,
        event: ProcessJobEvent,
    ) {
        if self.queue_events {
            lock(&self.events).push(event.clone());
        }
        send_event(
            events,
            wake,
            PluginHostEvent::JobEvent {
                plugin: self.plugin.clone(),
                event: event.name().to_string(),
                payload: event.payload(),
            },
        );
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}
