use std::time::Duration;

use kawi_plugin_sdk::exports::kawi::plugin::callback_handler::{Callback, Guest as CallbackGuest};
use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::task_runner::{self, TaskEvent, TaskState, TaskStatus};
use kawi_plugin_sdk::types::{Argument, Error};
use kawi_plugin_sdk::ui_message;

const LIST: &str = "list";
const RUN: &str = "run";
const START: &str = "start";
const DRAIN: &str = "drain";
const CANCEL: &str = "cancel";

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const CANCEL_DELAY: Duration = Duration::from_millis(50);
const MAX_WAIT: Duration = Duration::from_secs(4);

struct Plugin;

impl Guest for Plugin {
    fn init(_host_protocol: u32) -> Result<(), Error> {
        Ok(())
    }

    fn invoke_command(id: String, args: Vec<Argument>) -> Result<Argument, Error> {
        match id.as_str() {
            LIST => list(),
            RUN => run(&args),
            START => start(&args),
            DRAIN => drain(&args),
            CANCEL => cancel(&args),
            _ => message("unknown command"),
        }
    }

    fn on_event(_event: kawi_plugin_sdk::types::Event) {}
}

fn list() -> Result<Argument, Error> {
    let names = task_runner::list_tasks()?
        .into_iter()
        .map(|task| task.name)
        .collect::<Vec<_>>();
    message(&names.join(","))
}

fn run(args: &[Argument]) -> Result<Argument, Error> {
    let name = parse_name(args)?;
    let started = match task_runner::start(name) {
        Ok(started) => started,
        Err(error) => return message(&error_text(&error)),
    };
    let status = wait_for_status(started.job)?;
    let mut text = format!("{}|{}", started.job, format_status(&status));
    text.push_str(&drain_events(started.job)?);
    message(&text)
}

fn start(args: &[Argument]) -> Result<Argument, Error> {
    let name = parse_name(args)?;
    match task_runner::start(name) {
        Ok(started) => message(&started.job.to_string()),
        Err(error) => message(&error_text(&error)),
    }
}

fn drain(args: &[Argument]) -> Result<Argument, Error> {
    let job = parse_job(args)?;
    let status = wait_for_status(job)?;
    let mut text = format_status(&status);
    text.push_str(&drain_events(job)?);
    message(&text)
}

fn cancel(args: &[Argument]) -> Result<Argument, Error> {
    let name = parse_name(args)?;
    let started = match task_runner::start(name) {
        Ok(started) => started,
        Err(error) => return message(&error_text(&error)),
    };
    std::thread::sleep(CANCEL_DELAY);
    task_runner::cancel(started.job)?;
    let status = wait_for_status(started.job)?;
    message(&format_status(&status))
}

fn wait_for_status(job: u64) -> Result<TaskStatus, Error> {
    let mut waited = Duration::ZERO;
    loop {
        let status = task_runner::status(job)?;
        if status.state != TaskState::Running {
            return Ok(status);
        }
        if waited >= MAX_WAIT {
            return Err(error("task did not finish"));
        }
        std::thread::sleep(POLL_INTERVAL);
        waited += POLL_INTERVAL;
    }
}

fn drain_events(job: u64) -> Result<String, Error> {
    let mut text = String::new();
    for event in task_runner::poll(job)? {
        match event {
            TaskEvent::Output(output) => {
                let channel = match output.channel {
                    kawi_plugin_sdk::task_runner::TaskStream::Stdout => "out",
                    kawi_plugin_sdk::task_runner::TaskStream::Stderr => "err",
                };
                text.push_str(&format!("|{channel}={}", output.line));
            }
            TaskEvent::Finding(finding) => {
                text.push_str(&format!(
                    "|find={}:{}:{}:{}",
                    finding.line,
                    finding.column,
                    severity_name(finding.severity),
                    finding.message
                ));
            }
            TaskEvent::Exited(exited) => {
                text.push_str(&format!(
                    "|end={}{}{}",
                    state_name(exited.state),
                    exited.exit_code.unwrap_or(0),
                    if exited.success { "T" } else { "F" }
                ));
            }
        }
    }
    Ok(text)
}

fn format_status(status: &TaskStatus) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        state_name(status.state),
        status
            .exit_code
            .map_or_else(|| "-".to_string(), |code| code.to_string()),
        if status.success { "T" } else { "F" },
        if status.cancelled { "T" } else { "F" },
        status.findings,
        status.dropped_events,
        if status.duration_ms > 0 { "T" } else { "F" }
    )
}

fn state_name(state: TaskState) -> &'static str {
    match state {
        TaskState::Running => "R",
        TaskState::Exited => "E",
        TaskState::Cancelled => "C",
        TaskState::Failed => "F",
    }
}

fn severity_name(severity: kawi_plugin_sdk::task_runner::TaskSeverity) -> &'static str {
    use kawi_plugin_sdk::task_runner::TaskSeverity;
    match severity {
        TaskSeverity::Error => "E",
        TaskSeverity::Warning => "W",
        TaskSeverity::Information => "I",
        TaskSeverity::Hint => "H",
    }
}

fn parse_name(args: &[Argument]) -> Result<&str, Error> {
    match args.first() {
        Some(Argument::Text(value)) => Ok(value),
        _ => Err(error("missing task name")),
    }
}

fn parse_job(args: &[Argument]) -> Result<u64, Error> {
    match args.first() {
        Some(Argument::Text(value)) => value.parse().map_err(|_| error("invalid job")),
        _ => Err(error("missing job")),
    }
}

fn error_text(error: &Error) -> String {
    format!("err={}:{}", error.code, error.message)
}

fn message(text: &str) -> Result<Argument, Error> {
    ui_message::message(text)?;
    Ok(Argument::Null)
}

fn error(message: &str) -> Error {
    Error {
        code: kawi_plugin_sdk::INVALID_PARAMS,
        message: message.to_string(),
        capability: None,
    }
}

kawi_plugin_sdk::export!(Plugin);

impl CallbackGuest for Plugin {
    fn on_callback(_callback: Callback) -> Result<(), Error> {
        Ok(())
    }
}
