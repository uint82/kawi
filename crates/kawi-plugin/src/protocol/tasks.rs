use serde::{Deserialize, Serialize};

pub const TASK_OUTPUT_EVENT: &str = "task_output";
pub const TASK_FINDING_EVENT: &str = "task_finding";
pub const TASK_EXITED_EVENT: &str = "task_exited";
pub const MAX_TASKS_PER_PLUGIN: usize = 4;
pub const MAX_TASK_EVENT_QUEUE_ITEMS: usize = 10_000;
pub const MAX_TASK_EVENT_QUEUE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_TASK_LINE_BYTES: usize = 4 * 1024;
pub const MAX_TASK_FINDING_PATH_BYTES: usize = 4 * 1024;
pub const MAX_TASK_FINDING_MESSAGE_BYTES: usize = 4 * 1024;
pub const MAX_TASK_NAME_BYTES: usize = 256;
pub const TASK_RETENTION_SECS: u64 = 300;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Running,
    Exited,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskInfo {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskStartParams {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskStartResult {
    pub job: u64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskJobParams {
    pub job: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskStatusResult {
    pub job: u64,
    pub name: String,
    pub state: TaskState,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub cancelled: bool,
    pub duration_ms: u64,
    pub findings: u64,
    pub dropped_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskOutputEvent {
    pub job: u64,
    pub channel: TaskStream,
    pub line: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskFindingEvent {
    pub job: u64,
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub severity: TaskSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskExitedEvent {
    pub job: u64,
    pub state: TaskState,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub cancelled: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event", content = "payload")]
pub enum TaskEvent {
    #[serde(rename = "task_output")]
    Output(TaskOutputEvent),
    #[serde(rename = "task_finding")]
    Finding(TaskFindingEvent),
    #[serde(rename = "task_exited")]
    Exited(TaskExitedEvent),
}

impl TaskEvent {
    pub fn size(&self) -> usize {
        match self {
            Self::Output(event) => event.line.len() + 128,
            Self::Finding(event) => event.path.len() + event.message.len() + 128,
            Self::Exited(_) => 128,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_serialize_with_names_and_payloads() {
        let event = TaskEvent::Exited(TaskExitedEvent {
            job: 3,
            state: TaskState::Exited,
            exit_code: Some(1),
            success: false,
            cancelled: false,
            duration_ms: 12,
        });
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            serde_json::json!({
                "event": "task_exited",
                "payload": {
                    "job": 3,
                    "state": "exited",
                    "exit_code": 1,
                    "success": false,
                    "cancelled": false,
                    "duration_ms": 12
                }
            })
        );
        assert_eq!(event.size(), 128);
    }

    #[test]
    fn event_sizes_count_string_payloads() {
        let event = TaskEvent::Output(TaskOutputEvent {
            job: 1,
            channel: TaskStream::Stdout,
            line: "hello".to_string(),
            truncated: false,
        });
        assert_eq!(event.size(), 133);
    }
}
