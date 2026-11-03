use std::collections::BTreeMap;

use super::bindings::kawi::plugin::process_jobs::{
    self, JobEvent, JobExited, JobOutput, JobSignal, JobState, JobStatus, JobStream, OpenParams,
    StdinMode,
};
use super::bindings::kawi::plugin::types::Error;
use super::convert::error_from_rpc;
use super::state::{decode_json, serialize_params, WasmState};
use crate::protocol::{
    JobState as ProtocolJobState, JobStream as ProtocolJobStream, ProcessJobEvent,
    ProcessJobParams, ProcessOpenParams, ProcessOpenResult, ProcessSignal, ProcessSignalParams,
    ProcessStatusResult, ProcessWriteParams, RpcError, StdinMode as ProtocolStdinMode,
};

impl process_jobs::Host for WasmState {
    fn open(&mut self, params: OpenParams) -> Result<u64, Error> {
        let params = open_params(params)?;
        let value = self
            .bridge
            .request("process.open", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        let result: ProcessOpenResult = decode_json(value)?;
        Ok(result.job)
    }

    fn write(&mut self, job: u64, data: String) -> Result<(), Error> {
        let params = ProcessWriteParams { job, data };
        self.bridge
            .request("process.write", serialize_params(params)?)
            .map(|_| ())
            .map_err(error_from_rpc)
    }

    fn close_stdin(&mut self, job: u64) -> Result<(), Error> {
        let params = ProcessJobParams { job };
        self.bridge
            .request("process.close_stdin", serialize_params(params)?)
            .map(|_| ())
            .map_err(error_from_rpc)
    }

    fn signal(&mut self, job: u64, signal: JobSignal) -> Result<(), Error> {
        let params = ProcessSignalParams {
            job,
            signal: process_signal(signal),
        };
        self.bridge
            .request("process.signal", serialize_params(params)?)
            .map(|_| ())
            .map_err(error_from_rpc)
    }

    fn kill(&mut self, job: u64) -> Result<(), Error> {
        let params = ProcessJobParams { job };
        self.bridge
            .request("process.kill", serialize_params(params)?)
            .map(|_| ())
            .map_err(error_from_rpc)
    }

    fn status(&mut self, job: u64) -> Result<JobStatus, Error> {
        let params = ProcessJobParams { job };
        let value = self
            .bridge
            .request("process.status", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        let status: ProcessStatusResult = decode_json(value)?;
        Ok(job_status(status))
    }

    fn poll(&mut self, job: u64) -> Result<Vec<JobEvent>, Error> {
        let params = ProcessJobParams { job };
        let value = self
            .bridge
            .request("process.poll", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        let events: Vec<ProcessJobEvent> = decode_json(value)?;
        Ok(events.into_iter().map(job_event).collect())
    }
}

fn open_params(params: OpenParams) -> Result<ProcessOpenParams, Error> {
    let mut env = BTreeMap::new();
    for (key, value) in params.env {
        if env.insert(key.clone(), value).is_some() {
            return Err(error_from_rpc(RpcError::invalid_params(format!(
                "duplicate environment key: {key}"
            ))));
        }
    }
    Ok(ProcessOpenParams {
        command: params.command,
        args: params.args,
        cwd: params.cwd,
        env,
        stdin: stdin_mode(params.stdin),
    })
}

fn stdin_mode(mode: StdinMode) -> ProtocolStdinMode {
    match mode {
        StdinMode::Pipe => ProtocolStdinMode::Pipe,
        StdinMode::Null => ProtocolStdinMode::Null,
    }
}

fn process_signal(signal: JobSignal) -> ProcessSignal {
    match signal {
        JobSignal::Interrupt => ProcessSignal::Interrupt,
        JobSignal::Terminate => ProcessSignal::Terminate,
        JobSignal::Kill => ProcessSignal::Kill,
    }
}

fn job_status(status: ProcessStatusResult) -> JobStatus {
    JobStatus {
        state: job_state(status.state),
        exit_code: status.exit_code,
        signal: status.signal,
    }
}

fn job_event(event: ProcessJobEvent) -> JobEvent {
    match event {
        ProcessJobEvent::Output(event) => JobEvent::Output(JobOutput {
            job: event.job,
            channel: job_stream(event.stream),
            data: event.data,
            truncated: event.truncated,
        }),
        ProcessJobEvent::Exited(event) => JobEvent::Exited(JobExited {
            job: event.job,
            state: job_state(event.state),
            exit_code: event.exit_code,
            signal: event.signal,
        }),
    }
}

fn job_state(state: ProtocolJobState) -> JobState {
    match state {
        ProtocolJobState::Running => JobState::Running,
        ProtocolJobState::Exited => JobState::Exited,
        ProtocolJobState::Killed => JobState::Killed,
        ProtocolJobState::Failed => JobState::Failed,
    }
}

fn job_stream(stream: ProtocolJobStream) -> JobStream {
    match stream {
        ProtocolJobStream::Stdout => JobStream::Stdout,
        ProtocolJobStream::Stderr => JobStream::Stderr,
    }
}
