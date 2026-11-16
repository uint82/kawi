use super::bindings::kawi::plugin::task_runner::{
    self, StartedTask, TaskEvent, TaskExited, TaskFinding, TaskInfo, TaskOutput, TaskSeverity,
    TaskState, TaskStatus, TaskStream,
};
use super::bindings::kawi::plugin::types::Error;
use super::convert::error_from_rpc;
use super::state::{decode_json, serialize_params, WasmState};
use crate::protocol::{
    TaskEvent as ProtocolTaskEvent, TaskInfo as ProtocolTaskInfo, TaskJobParams,
    TaskSeverity as ProtocolTaskSeverity, TaskStartParams, TaskStartResult,
    TaskState as ProtocolTaskState, TaskStatusResult, TaskStream as ProtocolTaskStream,
};

impl task_runner::Host for WasmState {
    fn list_tasks(&mut self) -> Result<Vec<TaskInfo>, Error> {
        let value = self
            .bridge
            .request("task.list", serde_json::Value::Null)
            .map_err(error_from_rpc)?;
        let tasks: Vec<ProtocolTaskInfo> = decode_json(value)?;
        Ok(tasks.into_iter().map(task_info).collect())
    }

    fn start(&mut self, name: String) -> Result<StartedTask, Error> {
        let params = serialize_params(TaskStartParams { name })?;
        let value = self
            .bridge
            .request("task.start", params)
            .map_err(error_from_rpc)?;
        let started: TaskStartResult = decode_json(value)?;
        Ok(StartedTask {
            job: started.job,
            name: started.name,
        })
    }

    fn status(&mut self, job: u64) -> Result<TaskStatus, Error> {
        let params = serialize_params(TaskJobParams { job })?;
        let value = self
            .bridge
            .request("task.status", params)
            .map_err(error_from_rpc)?;
        let status: TaskStatusResult = decode_json(value)?;
        Ok(task_status(status))
    }

    fn poll(&mut self, job: u64) -> Result<Vec<TaskEvent>, Error> {
        let params = serialize_params(TaskJobParams { job })?;
        let value = self
            .bridge
            .request("task.poll", params)
            .map_err(error_from_rpc)?;
        let events: Vec<ProtocolTaskEvent> = decode_json(value)?;
        Ok(events.into_iter().map(task_event).collect())
    }

    fn cancel(&mut self, job: u64) -> Result<(), Error> {
        let params = serialize_params(TaskJobParams { job })?;
        self.bridge
            .request("task.cancel", params)
            .map(|_| ())
            .map_err(error_from_rpc)
    }
}

fn task_info(info: ProtocolTaskInfo) -> TaskInfo {
    TaskInfo {
        name: info.name,
        command: info.command,
        args: info.args,
    }
}

fn task_status(status: TaskStatusResult) -> TaskStatus {
    TaskStatus {
        job: status.job,
        name: status.name,
        state: task_state(status.state),
        exit_code: status.exit_code,
        success: status.success,
        cancelled: status.cancelled,
        duration_ms: status.duration_ms,
        findings: status.findings,
        dropped_events: status.dropped_events,
    }
}

fn task_event(event: ProtocolTaskEvent) -> TaskEvent {
    match event {
        ProtocolTaskEvent::Output(event) => TaskEvent::Output(TaskOutput {
            job: event.job,
            channel: task_stream(event.channel),
            line: event.line,
            truncated: event.truncated,
        }),
        ProtocolTaskEvent::Finding(event) => TaskEvent::Finding(TaskFinding {
            job: event.job,
            path: event.path,
            line: event.line,
            column: event.column,
            severity: task_severity(event.severity),
            message: event.message,
        }),
        ProtocolTaskEvent::Exited(event) => TaskEvent::Exited(TaskExited {
            job: event.job,
            state: task_state(event.state),
            exit_code: event.exit_code,
            success: event.success,
            cancelled: event.cancelled,
            duration_ms: event.duration_ms,
        }),
    }
}

fn task_state(state: ProtocolTaskState) -> TaskState {
    match state {
        ProtocolTaskState::Running => TaskState::Running,
        ProtocolTaskState::Exited => TaskState::Exited,
        ProtocolTaskState::Cancelled => TaskState::Cancelled,
        ProtocolTaskState::Failed => TaskState::Failed,
    }
}

fn task_stream(stream: ProtocolTaskStream) -> TaskStream {
    match stream {
        ProtocolTaskStream::Stdout => TaskStream::Stdout,
        ProtocolTaskStream::Stderr => TaskStream::Stderr,
    }
}

fn task_severity(severity: ProtocolTaskSeverity) -> TaskSeverity {
    match severity {
        ProtocolTaskSeverity::Error => TaskSeverity::Error,
        ProtocolTaskSeverity::Warning => TaskSeverity::Warning,
        ProtocolTaskSeverity::Information => TaskSeverity::Information,
        ProtocolTaskSeverity::Hint => TaskSeverity::Hint,
    }
}
