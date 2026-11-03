use std::process::Child;
use std::sync::Mutex;

use crate::protocol::{JobState, ProcessSignal, RpcError, JOB_CLOSED};

use super::monitor::JobStatus;
use super::{lock, JobHandle};

pub(super) fn ensure_running(handle: &JobHandle) -> Result<(), RpcError> {
    if handle.state() == JobState::Running {
        Ok(())
    } else {
        Err(RpcError::new(JOB_CLOSED, "job is not running"))
    }
}

pub(super) fn terminate(handle: &JobHandle) {
    let mut child = lock(&handle.child);
    if child.try_wait().ok().flatten().is_some() {
        return;
    }
    kawi_process::terminate(&mut child);
    set_state(&handle.status, JobState::Killed);
}

pub(super) fn set_state(status: &Mutex<JobStatus>, state: JobState) {
    lock(status).state = state;
}

#[cfg(unix)]
pub(super) fn send_signal(child: &mut Child, signal: ProcessSignal) -> Result<(), RpcError> {
    kawi_process::signal_group(child.id(), platform_signal(signal));
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn send_signal(child: &mut Child, signal: ProcessSignal) -> Result<(), RpcError> {
    if signal != ProcessSignal::Kill {
        return Err(RpcError::unavailable(
            "signal is not supported on this platform",
        ));
    }
    child
        .kill()
        .map_err(|error| RpcError::unavailable(error.to_string()))
}

#[cfg(unix)]
fn platform_signal(signal: ProcessSignal) -> kawi_process::ProcessSignal {
    match signal {
        ProcessSignal::Interrupt => kawi_process::ProcessSignal::Interrupt,
        ProcessSignal::Terminate => kawi_process::ProcessSignal::Terminate,
        ProcessSignal::Kill => kawi_process::ProcessSignal::Kill,
    }
}
