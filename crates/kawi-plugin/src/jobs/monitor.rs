use std::process::ExitStatus;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::host::PluginHostEvent;
use crate::protocol::{JobExitedEvent, JobState, JOB_EXITED_EVENT};

use super::output::OutputTracker;
use super::{lock, send_job_event, JobHandle};

const MONITOR_INTERVAL: Duration = Duration::from_millis(20);
const OUTPUT_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) struct JobStatus {
    pub(super) state: JobState,
    pub(super) exit_code: Option<i32>,
    pub(super) signal: Option<i32>,
    pub(super) exited_at: Option<Instant>,
}

impl JobStatus {
    pub(super) fn new() -> Self {
        Self {
            state: JobState::Running,
            exit_code: None,
            signal: None,
            exited_at: None,
        }
    }
}

struct ExitRecord {
    state: JobState,
    exit_code: Option<i32>,
    signal: Option<i32>,
}

enum MonitorOutcome {
    Running,
    Finished(ExitRecord),
}

pub(super) fn spawn_monitor(
    events: &std::sync::mpsc::Sender<PluginHostEvent>,
    wake: &std::sync::mpsc::Sender<()>,
    job: u64,
    handle: &JobHandle,
    tracker: Arc<OutputTracker>,
) {
    let events = events.clone();
    let wake = wake.clone();
    let handle = handle.clone();
    thread::spawn(move || monitor_loop(&events, &wake, job, &handle, &tracker));
}

fn monitor_loop(
    events: &std::sync::mpsc::Sender<PluginHostEvent>,
    wake: &std::sync::mpsc::Sender<()>,
    job: u64,
    handle: &JobHandle,
    tracker: &OutputTracker,
) {
    loop {
        let outcome = {
            let mut child = lock(&handle.child);
            match child.try_wait() {
                Ok(Some(status)) => MonitorOutcome::Finished(record_exit(handle, &status)),
                Ok(None) => MonitorOutcome::Running,
                Err(_) => MonitorOutcome::Finished(record_failure(handle)),
            }
        };
        match outcome {
            MonitorOutcome::Running => thread::sleep(MONITOR_INTERVAL),
            MonitorOutcome::Finished(record) => {
                tracker.wait(OUTPUT_DRAIN_TIMEOUT);
                publish_exit(handle, events, wake, job, record);
                return;
            }
        }
    }
}

fn record_exit(handle: &JobHandle, status: &ExitStatus) -> ExitRecord {
    let exit_code = status.code();
    let signal = exit_signal(status);
    let mut current = lock(&handle.status);
    if current.state != JobState::Killed {
        current.state = JobState::Exited;
    }
    current.exit_code = exit_code;
    current.signal = signal;
    current.exited_at = Some(Instant::now());
    ExitRecord {
        state: current.state,
        exit_code,
        signal,
    }
}

fn record_failure(handle: &JobHandle) -> ExitRecord {
    let mut current = lock(&handle.status);
    current.state = JobState::Failed;
    current.exit_code = None;
    current.signal = None;
    current.exited_at = Some(Instant::now());
    ExitRecord {
        state: current.state,
        exit_code: None,
        signal: None,
    }
}

fn publish_exit(
    handle: &JobHandle,
    events: &std::sync::mpsc::Sender<PluginHostEvent>,
    wake: &std::sync::mpsc::Sender<()>,
    job: u64,
    record: ExitRecord,
) {
    handle.stdin.close_after_exit();
    send_job_event(
        events,
        wake,
        &handle.plugin,
        JOB_EXITED_EVENT,
        json!(JobExitedEvent {
            job,
            state: record.state,
            exit_code: record.exit_code,
            signal: record.signal,
        }),
    );
}

#[cfg(unix)]
fn exit_signal(status: &ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn exit_signal(_status: &ExitStatus) -> Option<i32> {
    None
}
