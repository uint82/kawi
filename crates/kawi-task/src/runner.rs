use crate::config::TaskSpec;
use crate::parser::{Finding, OutputStream, Parser};
use anyhow::{Context, Result};
use kawi_process::{signal_group, ProcessSignal};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const READER_BUFFER: usize = 1_024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const CANCEL_GRACE: Duration = Duration::from_millis(500);
const EXIT_DRAIN: Duration = Duration::from_millis(250);
const MAX_OUTPUT_LINE_CHARS: usize = 4_096;

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

struct RawLine {
    stream: OutputStream,
    line: String,
}

fn sanitize_line(text: &str) -> String {
    let mut result = String::new();
    let mut characters = text.chars().peekable();
    let mut length = 0;
    while let Some(character) = characters.next() {
        if length >= MAX_OUTPUT_LINE_CHARS {
            break;
        }
        match character {
            '\u{1b}' => skip_escape(&mut characters),
            '\t' => {
                let spaces = (MAX_OUTPUT_LINE_CHARS - length).min(4);
                result.push_str(&" ".repeat(spaces));
                length = length.saturating_add(spaces);
            }
            '\r' => {}
            control if control.is_control() => {}
            text => {
                result.push(text);
                length = length.saturating_add(1);
            }
        }
    }
    result
}

fn skip_escape(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    match characters.next() {
        Some('[') => {
            for character in characters.by_ref() {
                if ('@'..='~').contains(&character) {
                    break;
                }
            }
        }
        Some(']') => {
            while let Some(character) = characters.next() {
                if character == '\u{7}' {
                    break;
                }
                if character == '\u{1b}' && characters.peek() == Some(&'\\') {
                    characters.next();
                    break;
                }
            }
        }
        _ => {}
    }
}

fn task_directory(spec: &TaskSpec, root: &Path) -> PathBuf {
    match spec.cwd.as_deref() {
        Some(cwd) if cwd.is_absolute() => cwd.to_path_buf(),
        Some(cwd) => root.join(cwd),
        None => root.to_path_buf(),
    }
}

fn spawn_reader<R: Read + Send + 'static>(
    name: &str,
    reader: R,
    stream: OutputStream,
    sender: SyncSender<RawLine>,
) -> std::io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            let mut reader = BufReader::new(reader);
            let mut bytes = Vec::new();
            loop {
                bytes.clear();
                match reader.read_until(b'\n', &mut bytes) {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {
                        if bytes.last() == Some(&b'\n') {
                            bytes.pop();
                        }
                        if bytes.last() == Some(&b'\r') {
                            bytes.pop();
                        }
                        let line = sanitize_line(&String::from_utf8_lossy(&bytes));
                        if sender.send(RawLine { stream, line }).is_err() {
                            return;
                        }
                    }
                }
            }
        })
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
mod tests {
    use super::*;
    use crate::parser::ParserSpec;
    use std::collections::BTreeMap;

    fn spec(command: &str, args: &[&str], parser: ParserSpec) -> TaskSpec {
        TaskSpec {
            name: "test".to_string(),
            command: command.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
            cwd: None,
            env: BTreeMap::new(),
            parser,
        }
    }

    fn wait(runner: &TaskRunner) -> Vec<TaskEvent> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut events = Vec::new();
        while Instant::now() < deadline {
            while let Some(event) = runner.try_recv() {
                let finished = matches!(event, TaskEvent::Finished { .. });
                events.push(event);
                if finished {
                    return events;
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("task did not finish");
    }

    #[test]
    fn strips_terminal_sequences_and_bounds_lines() {
        let text = format!("\u{1b}[31merror\u{1b}[0m\tvalue\r{}", "x".repeat(5_000));
        let line = sanitize_line(&text);
        assert!(line.starts_with("error    value"));
        assert!(!line.contains('\u{1b}'));
        assert_eq!(line.chars().count(), MAX_OUTPUT_LINE_CHARS);
    }

    #[test]
    #[cfg(unix)]
    fn streams_stdout_and_stderr() {
        let root = std::env::temp_dir();
        let wake = mpsc::channel().0;
        let runner = TaskRunner::start(
            &spec(
                "sh",
                &["-c", "printf 'one\\n'; printf 'two\\n' >&2"],
                ParserSpec::None,
            ),
            &root,
            wake,
        )
        .unwrap();
        let events = wait(&runner);
        let lines: Vec<(OutputStream, String)> = events
            .iter()
            .filter_map(|event| match event {
                TaskEvent::Output { stream, line } => Some((*stream, line.clone())),
                _ => None,
            })
            .collect();
        assert!(lines.contains(&(OutputStream::Stdout, "one".to_string())));
        assert!(lines.contains(&(OutputStream::Stderr, "two".to_string())));
        assert!(matches!(
            events.last(),
            Some(TaskEvent::Finished {
                status: TaskStatus { success: true, .. },
                ..
            })
        ));
    }

    #[test]
    #[cfg(unix)]
    fn reports_findings_and_failure() {
        let root = std::env::temp_dir();
        let wake = mpsc::channel().0;
        let runner = TaskRunner::start(
            &spec(
                "sh",
                &["-c", "printf 'a.go:2:3: bad\\n'; exit 1"],
                ParserSpec::Go,
            ),
            &root,
            wake,
        )
        .unwrap();
        let events = wait(&runner);
        let finding = events
            .iter()
            .find_map(|event| match event {
                TaskEvent::Finding(finding) => Some(finding),
                _ => None,
            })
            .unwrap();
        assert_eq!(finding.path, root.join("a.go"));
        assert_eq!(finding.line, 2);
        assert!(matches!(
            events.last(),
            Some(TaskEvent::Finished {
                status: TaskStatus { success: false, .. },
                description,
            }) if description == "exit 1"
        ));
    }

    #[test]
    #[cfg(unix)]
    fn cancels_the_whole_process_group() {
        let root = std::env::temp_dir();
        let wake = mpsc::channel().0;
        let runner = TaskRunner::start(
            &spec("sh", &["-c", "sleep 10 & wait"], ParserSpec::None),
            &root,
            wake,
        )
        .unwrap();
        runner.cancel();
        let events = wait(&runner);
        assert!(matches!(
            events.last(),
            Some(TaskEvent::Finished {
                status: TaskStatus {
                    cancelled: true,
                    success: false,
                    ..
                },
                ..
            })
        ));
    }
}
