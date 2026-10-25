use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::thread::{self, JoinHandle};

use super::parser::{LogEntry, LogOptions};

const FORMAT: &str = "%H%x00%h%x00%aN%x00%aI%x00%s%x00%D%x1e";
const FIELD_COUNT: usize = 6;
const BATCH_SIZE: usize = 128;
const CHANNEL_SIZE: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEvent {
    Entries(Vec<LogEntry>),
    Done(Option<String>),
}

pub struct LogStream {
    child: Option<Child>,
    receiver: Option<Receiver<LogEvent>>,
    worker: Option<JoinHandle<()>>,
}

impl LogStream {
    pub fn start(root: &Path, options: &LogOptions, wake: Sender<()>) -> Result<Self> {
        let mut child = command(root, options)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to start git log")?;
        let stdout = child.stdout.take().context("missing git log stdout")?;
        let stderr = child.stderr.take().context("missing git log stderr")?;
        let (sender, receiver) = mpsc::sync_channel(CHANNEL_SIZE);
        let worker = spawn_reader(stdout, stderr, sender, wake)?;
        Ok(Self {
            child: Some(child),
            receiver: Some(receiver),
            worker: Some(worker),
        })
    }

    pub fn try_recv(&self) -> Option<LogEvent> {
        self.receiver.as_ref()?.try_recv().ok()
    }

    pub fn is_done(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }
}

impl Drop for LogStream {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.receiver.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn command(root: &Path, options: &LogOptions) -> Command {
    let mut command = Command::new("git");
    command
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(root)
        .arg("log")
        .arg("-z")
        .arg("--no-color")
        .arg(format!("--format={FORMAT}"))
        .env("LC_ALL", "C")
        .env("GIT_OPTIONAL_LOCKS", "0");
    if let Some(grep) = options.grep.as_deref() {
        command.arg("--grep").arg(grep);
    }
    if let Some(author) = options.author.as_deref() {
        command.arg("--author").arg(author);
    }
    if let Some(path) = options.path.as_deref() {
        command.arg("--follow").arg("--").arg(path);
    }
    command
}

fn spawn_reader(
    stdout: std::process::ChildStdout,
    stderr: ChildStderr,
    sender: SyncSender<LogEvent>,
    wake: Sender<()>,
) -> Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("kawi-git-log".to_string())
        .spawn(move || read_stream(stdout, stderr, sender, wake))
        .context("spawn git log reader")
}

fn read_stream(
    stdout: ChildStdout,
    stderr: ChildStderr,
    sender: SyncSender<LogEvent>,
    wake: Sender<()>,
) {
    let stderr_worker = spawn_stderr(stderr).ok();
    let mut reader = BufReader::new(stdout);
    let mut pending = Vec::new();
    let mut batch = Vec::new();
    let mut buffer = Vec::new();
    let mut error = None;
    loop {
        buffer.clear();
        match reader.read_until(0, &mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                let token = buffer.strip_suffix(&[0]).unwrap_or(&buffer);
                if token.is_empty() {
                    continue;
                }
                pending.push(token.to_vec());
                if pending.len() == FIELD_COUNT {
                    match parse_entry(&pending) {
                        Some(entry) => batch.push(entry),
                        None => {
                            error = Some("invalid git log record".to_string());
                            break;
                        }
                    }
                    pending.clear();
                    if batch.len() >= BATCH_SIZE {
                        if sender
                            .send(LogEvent::Entries(std::mem::take(&mut batch)))
                            .is_err()
                        {
                            return;
                        }
                        let _ = wake.send(());
                    }
                }
            }
            Err(error_value) => {
                error = Some(error_value.to_string());
                break;
            }
        }
    }
    if !batch.is_empty() {
        let _ = sender.send(LogEvent::Entries(batch));
        let _ = wake.send(());
    }
    let stderr = stderr_worker
        .and_then(|worker| worker.join().ok())
        .filter(|text| !text.is_empty());
    if error.is_none() {
        error = stderr;
    }
    if !pending.is_empty() && error.is_none() {
        error = Some("truncated git log record".to_string());
    }
    let _ = sender.send(LogEvent::Done(error));
    let _ = wake.send(());
}

fn spawn_stderr(stderr: ChildStderr) -> Result<JoinHandle<String>> {
    thread::Builder::new()
        .name("kawi-git-log-stderr".to_string())
        .spawn(move || read_stderr(stderr))
        .context("spawn git log stderr reader")
}

fn read_stderr(mut stderr: impl Read) -> String {
    let mut text = String::new();
    let _ = stderr.read_to_string(&mut text);
    text.trim().to_string()
}

fn parse_entry(fields: &[Vec<u8>]) -> Option<LogEntry> {
    Some(LogEntry {
        oid: text(&fields[0])?,
        short_oid: text(&fields[1])?,
        author: text(&fields[2])?,
        date: text(&fields[3])?,
        subject: text(&fields[4])?,
        decorations: text(fields[5].strip_suffix(b"\x1e")?)?,
    })
}

fn text(bytes: &[u8]) -> Option<String> {
    std::str::from_utf8(bytes).ok().map(str::to_string)
}
