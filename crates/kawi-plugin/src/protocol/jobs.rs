use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const JOB_OUTPUT_EVENT: &str = "job_output";
pub const JOB_EXITED_EVENT: &str = "job_exited";
pub const MAX_JOBS_PER_PLUGIN: usize = 16;
pub const MAX_JOB_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_JOB_STDIN_BYTES: usize = 1024 * 1024;
pub const MAX_JOB_ENV_ENTRIES: usize = 64;
pub const MAX_JOB_ENV_KEY_BYTES: usize = 1024;
pub const MAX_JOB_ENV_VALUE_BYTES: usize = 32 * 1024;
pub const MAX_JOB_ENV_BYTES: usize = 128 * 1024;
pub const MAX_JOB_ARGUMENTS: usize = 256;
pub const MAX_JOB_ARGUMENT_BYTES: usize = 64 * 1024;
pub const MAX_JOB_ARGUMENT_TOTAL_BYTES: usize = 1024 * 1024;
pub const MAX_JOB_EVENT_QUEUE_ITEMS: usize = 1024;
pub const MAX_JOB_EVENT_QUEUE_BYTES: usize = MAX_JOB_OUTPUT_BYTES + 64 * 1024;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StdinMode {
    #[default]
    Pipe,
    Null,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessSignal {
    Interrupt,
    Terminate,
    Kill,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessOpenParams {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub stdin: StdinMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessOpenResult {
    pub job: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessWriteParams {
    pub job: u64,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessJobParams {
    pub job: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessSignalParams {
    pub job: u64,
    pub signal: ProcessSignal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Running,
    Exited,
    Killed,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessStatusResult {
    pub state: JobState,
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub signal: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobOutputEvent {
    pub job: u64,
    pub stream: JobStream,
    pub data: String,
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobExitedEvent {
    pub job: u64,
    pub state: JobState,
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub signal: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event", content = "payload")]
pub enum ProcessJobEvent {
    #[serde(rename = "job_output")]
    Output(JobOutputEvent),
    #[serde(rename = "job_exited")]
    Exited(JobExitedEvent),
}

impl ProcessJobEvent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Output(_) => JOB_OUTPUT_EVENT,
            Self::Exited(_) => JOB_EXITED_EVENT,
        }
    }

    pub fn payload(&self) -> serde_json::Value {
        match self {
            Self::Output(event) => serde_json::to_value(event),
            Self::Exited(event) => serde_json::to_value(event),
        }
        .expect("job event serializes")
    }

    pub fn size(&self) -> usize {
        match self {
            Self::Output(event) => event.data.len() + 64,
            Self::Exited(_) => 64,
        }
    }
}
