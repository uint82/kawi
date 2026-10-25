use crate::config::TaskSpec;
use crate::parser::{Finding, OutputStream, Parser};
use anyhow::{Context, Result};
use kawi_process::{signal_group, ProcessSignal};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

mod reader;

use reader::{spawn_reader, RawLine};

const READER_BUFFER: usize = 1_024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const CANCEL_GRACE: Duration = Duration::from_millis(500);
const EXIT_DRAIN: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TaskOptions {
    pub detached: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskStatus {
    pub success: bool,
    pub cancelled: bool,
    pub duration: Duration,
}

#[derive(Debug)]
pub enum TaskEvent {
    Output {
        stream: OutputStream,
        line: String,
    },
    Finding(Finding),
    Finished {
        status: TaskStatus,
        description: String,
    },
}

pub struct TaskRunner {
    events: Receiver<TaskEvent>,
    cancel: Arc<AtomicBool>,
    force_kill: Arc<AtomicBool>,
    monitor: Option<JoinHandle<()>>,
}

impl TaskRunner {
    pub fn start(spec: &TaskSpec, root: &Path, wake: Sender<()>) -> Result<Self> {
        Self::start_with(spec, root, wake, TaskOptions::default())
    }

    pub fn start_with(
        spec: &TaskSpec,
        root: &Path,
        wake: Sender<()>,
        options: TaskOptions,
    ) -> Result<Self> {
        let cwd = task_directory(spec, root);
        let mut command = Command::new(&spec.command);
        command
            .args(&spec.args)
            .envs(&spec.env)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if options.detached {
            kawi_process::configure_session(&mut command);
        } else {
            kawi_process::configure(&mut command);
        }
        let mut child = command
            .spawn()
            .with_context(|| format!("start {}", spec.command))?;
        let stdout = child.stdout.take().context("open task stdout")?;
        let stderr = child.stderr.take().context("open task stderr")?;
        let pid = child.id();
        let (raw_sender, raw_receiver) = mpsc::sync_channel(READER_BUFFER);
        let stdout_reader = spawn_reader(
            "kawi-task-stdout",
            stdout,
            OutputStream::Stdout,
            raw_sender.clone(),
        );
        let stderr_reader =
            spawn_reader("kawi-task-stderr", stderr, OutputStream::Stderr, raw_sender);
        if stdout_reader.is_err() || stderr_reader.is_err() {
            kawi_process::terminate(&mut child);
            return Err(anyhow::anyhow!("start task output readers"));
        }
        let (sender, events) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let force_kill = Arc::new(AtomicBool::new(false));
        let parser = Parser::new(spec.parser.clone());
        let finding_root = cwd;
        let monitor_cancel = Arc::clone(&cancel);
        let monitor_kill = Arc::clone(&force_kill);
        let start = Instant::now();
        let monitor = match thread::Builder::new()
            .name("kawi-task-monitor".to_string())
            .spawn(move || {
                monitor(
                    child,
                    raw_receiver,
                    parser,
                    sender,
                    wake,
                    monitor_cancel,
                    monitor_kill,
                    finding_root,
                    start,
                );
            }) {
            Ok(monitor) => monitor,
            Err(error) => {
                signal_group(pid, ProcessSignal::Kill);
                return Err(anyhow::Error::new(error).context("spawn task monitor"));
            }
        };
        Ok(Self {
            events,
            cancel,
            force_kill,
            monitor: Some(monitor),
        })
    }

    pub fn try_recv(&self) -> Option<TaskEvent> {
        self.events.try_recv().ok()
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

impl Drop for TaskRunner {
    fn drop(&mut self) {
        self.force_kill.store(true, Ordering::SeqCst);
        if let Some(monitor) = self.monitor.take() {
            let _ = monitor.join();
        }
    }
}

fn task_directory(spec: &TaskSpec, root: &Path) -> PathBuf {
    match spec.cwd.as_deref() {
        Some(cwd) if cwd.is_absolute() => cwd.to_path_buf(),
        Some(cwd) => root.join(cwd),
        None => root.to_path_buf(),
    }
}

#[allow(clippy::too_many_arguments)]
fn monitor(
    mut child: Child,
    raw: Receiver<RawLine>,
    mut parser: Parser,
    events: Sender<TaskEvent>,
    wake: Sender<()>,
    cancel: Arc<AtomicBool>,
    force_kill: Arc<AtomicBool>,
    root: PathBuf,
    start: Instant,
) {
    let mut deadline = None;
    let mut exited = None;
    let mut exit_at = None;
    let mut closed = false;
    loop {
        if exited.is_none() && cancel.load(Ordering::SeqCst) && deadline.is_none() {
            signal_group(child.id(), ProcessSignal::Terminate);
            deadline = Some(Instant::now() + CANCEL_GRACE);
        }
        if exited.is_none()
            && (force_kill.load(Ordering::SeqCst)
                || deadline.is_some_and(|deadline| Instant::now() >= deadline))
        {
            signal_group(child.id(), ProcessSignal::Kill);
            let _ = child.kill();
            deadline = None;
            force_kill.store(false, Ordering::SeqCst);
        }
        match raw.recv_timeout(POLL_INTERVAL) {
            Ok(raw) => {
                if !emit_line(&events, &wake, &mut parser, &root, raw) {
                    return;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => closed = true,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if exited.is_none() {
            exited = child.try_wait().ok().flatten();
            if exited.is_some() {
                exit_at = Some(Instant::now());
            }
        }
        if closed && exited.is_some() {
            break;
        }
        if exit_at.is_some_and(|exit_at| exit_at.elapsed() > EXIT_DRAIN) {
            break;
        }
    }
    let findings = parser.finish();
    for finding in findings {
        if events.send(TaskEvent::Finding(finding)).is_err() {
            return;
        }
    }
    let cancelled = cancel.load(Ordering::SeqCst);
    let status = child.wait().ok().or(exited);
    let success = !cancelled && status.as_ref().is_some_and(|status| status.success());
    let event = TaskEvent::Finished {
        status: TaskStatus {
            success,
            cancelled,
            duration: start.elapsed(),
        },
        description: finish_description(cancelled, status),
    };
    let _ = events.send(event);
    let _ = wake.send(());
}

fn emit_line(
    events: &Sender<TaskEvent>,
    wake: &Sender<()>,
    parser: &mut Parser,
    root: &Path,
    raw: RawLine,
) -> bool {
    if events
        .send(TaskEvent::Output {
            stream: raw.stream,
            line: raw.line.clone(),
        })
        .is_err()
    {
        return false;
    }
    for finding in parser.parse_line(raw.stream, &raw.line) {
        if events
            .send(TaskEvent::Finding(resolve_finding(finding, root)))
            .is_err()
        {
            return false;
        }
    }
    let _ = wake.send(());
    true
}

fn resolve_finding(mut finding: Finding, root: &Path) -> Finding {
    if !finding.path.is_absolute() {
        finding.path = root.join(finding.path);
    }
    finding
}

fn finish_description(cancelled: bool, status: Option<std::process::ExitStatus>) -> String {
    if cancelled {
        return "cancelled".to_string();
    }
    match status.and_then(|status| status.code()) {
        Some(code) => format!("exit {code}"),
        None => "terminated".to_string(),
    }
}

#[cfg(test)]
mod tests;
