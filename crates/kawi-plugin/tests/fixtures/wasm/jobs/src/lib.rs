use kawi_plugin_sdk::exports::kawi::plugin::lifecycle::Guest;
use kawi_plugin_sdk::process_jobs::{
    self, JobEvent, JobSignal, JobState, JobStatus, JobStream, OpenParams, StdinMode,
};
use kawi_plugin_sdk::types::{Argument, Error, Event};
use kawi_plugin_sdk::{ui_message, INVALID_PARAMS};

const START: &str = "start";
const ECHO: &str = "echo";
const SIGNAL: &str = "signal";
const KILL: &str = "kill";
const DRAIN: &str = "drain";

struct Plugin;

impl Guest for Plugin {
    fn init(_host_protocol: u32) -> Result<(), Error> {
        Ok(())
    }

    fn invoke_command(id: String, args: Vec<Argument>) -> Result<Argument, Error> {
        match id.as_str() {
            START => start(),
            ECHO => echo(),
            SIGNAL => signal(),
            KILL => kill(),
            DRAIN => drain(&args),
            _ => Err(error("unknown command")),
        }
    }

    fn on_event(_event: Event) {}
}

fn start() -> Result<Argument, Error> {
    open_and_report(&OpenParams {
        command: "/bin/sh".to_string(),
        args: vec![
            "-c".to_string(),
            "printf hello; printf err >&2; exit 7".to_string(),
        ],
        cwd: None,
        env: Vec::new(),
        stdin: StdinMode::Null,
    })
}

fn echo() -> Result<Argument, Error> {
    let job = process_jobs::open(&OpenParams {
        command: "/bin/cat".to_string(),
        args: Vec::new(),
        cwd: None,
        env: Vec::new(),
        stdin: StdinMode::Pipe,
    })?;
    process_jobs::write(job, "hello\n")?;
    process_jobs::close_stdin(job)?;
    message(&job.to_string())?;
    Ok(Argument::Null)
}

fn signal() -> Result<Argument, Error> {
    let job = open_sleep()?;
    process_jobs::signal(job, JobSignal::Terminate)?;
    message(&job.to_string())?;
    Ok(Argument::Null)
}

fn kill() -> Result<Argument, Error> {
    let job = open_sleep()?;
    process_jobs::kill(job)?;
    message(&job.to_string())?;
    Ok(Argument::Null)
}

fn open_sleep() -> Result<u64, Error> {
    process_jobs::open(&OpenParams {
        command: "/bin/sleep".to_string(),
        args: vec!["5".to_string()],
        cwd: None,
        env: Vec::new(),
        stdin: StdinMode::Null,
    })
}

fn open_and_report(params: &OpenParams) -> Result<Argument, Error> {
    let job = process_jobs::open(params)?;
    message(&job.to_string())?;
    Ok(Argument::Null)
}

fn drain(args: &[Argument]) -> Result<Argument, Error> {
    let job = parse_job(args)?;
    let mut status: JobStatus = process_jobs::status(job)?;
    while status.state == JobState::Running {
        std::thread::sleep(std::time::Duration::from_millis(10));
        status = process_jobs::status(job)?;
    }
    let mut text = String::new();
    for event in process_jobs::poll(job)? {
        match event {
            JobEvent::Output(output) => {
                text.push_str(channel_name(output.channel));
                text.push(':');
                text.push_str(&output.data);
                text.push(';');
            }
            JobEvent::Exited(exited) => {
                text.push_str(&format!(
                    "exit:{:?}:{:?}:{:?};",
                    exited.state, exited.exit_code, exited.signal
                ));
            }
        }
    }
    message(&text)?;
    Ok(Argument::Null)
}

fn parse_job(args: &[Argument]) -> Result<u64, Error> {
    match args.first() {
        Some(Argument::Text(value)) => value.parse().map_err(|_| error("invalid job")),
        _ => Err(error("missing job")),
    }
}

fn channel_name(channel: JobStream) -> &'static str {
    match channel {
        JobStream::Stdout => "out",
        JobStream::Stderr => "err",
    }
}

fn message(text: &str) -> Result<(), Error> {
    ui_message::message(text)
}

fn error(message: &str) -> Error {
    Error {
        code: INVALID_PARAMS,
        message: message.to_string(),
        capability: None,
    }
}

kawi_plugin_sdk::export!(Plugin);
