use crate::protocol::Message;
use crate::service::ServiceInput;
use anyhow::{Context, Result};
use kawi_jsonrpc::framing;
use std::io::{BufRead, BufReader, BufWriter};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::thread;

const MAX_LOG_LINE: usize = 8 * 1024;

#[derive(Debug)]
pub(crate) enum ClientEvent {
    Message {
        generation: u64,
        message: Message,
    },
    ReaderClosed {
        generation: u64,
        error: Option<String>,
    },
    WriterClosed {
        generation: u64,
        error: String,
    },
    Stderr {
        generation: u64,
        line: String,
    },
}

pub(crate) struct Client {
    process: Child,
    writer: Sender<Message>,
}

impl Client {
    pub(crate) fn spawn(
        generation: u64,
        command: &str,
        args: &[String],
        env: &std::collections::BTreeMap<String, String>,
        cwd: Option<&std::path::Path>,
        input: Sender<ServiceInput>,
    ) -> Result<Self> {
        let executable = kawi_process::find_executable(command)
            .ok_or_else(|| anyhow::anyhow!("debug adapter {command} not found in PATH"))?;
        let mut process = Command::new(executable);
        process
            .args(args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(cwd) = cwd {
            process.current_dir(cwd);
        }
        kawi_process::configure(&mut process);
        let mut child = process
            .spawn()
            .with_context(|| format!("spawn debug adapter {command}"))?;
        let stdin = child.stdin.take().context("debug adapter stdin")?;
        let stdout = child.stdout.take().context("debug adapter stdout")?;
        let stderr = child.stderr.take().context("debug adapter stderr")?;
        let (writer, writer_rx) = mpsc::channel();
        spawn_writer(generation, stdin, writer_rx, input.clone())?;
        spawn_reader(generation, stdout, input.clone())?;
        spawn_stderr(generation, stderr, input)?;
        Ok(Self {
            process: child,
            writer,
        })
    }

    pub(crate) fn send(&self, message: Message) -> bool {
        self.writer.send(message).is_ok()
    }

    pub(crate) fn terminate(&mut self) {
        kawi_process::terminate(&mut self.process);
    }
}

fn spawn_writer(
    generation: u64,
    stdin: std::process::ChildStdin,
    receiver: mpsc::Receiver<Message>,
    input: Sender<ServiceInput>,
) -> Result<()> {
    thread::Builder::new()
        .name("kawi-dap-write".to_string())
        .spawn(move || write_loop(generation, stdin, receiver, input))
        .context("spawn dap writer")?;
    Ok(())
}

fn spawn_reader(
    generation: u64,
    stdout: std::process::ChildStdout,
    input: Sender<ServiceInput>,
) -> Result<()> {
    thread::Builder::new()
        .name("kawi-dap-read".to_string())
        .spawn(move || read_loop(generation, stdout, input))
        .context("spawn dap reader")?;
    Ok(())
}

fn spawn_stderr(
    generation: u64,
    stderr: std::process::ChildStderr,
    input: Sender<ServiceInput>,
) -> Result<()> {
    thread::Builder::new()
        .name("kawi-dap-log".to_string())
        .spawn(move || stderr_loop(generation, stderr, input))
        .context("spawn dap stderr reader")?;
    Ok(())
}

fn write_loop(
    generation: u64,
    stdin: std::process::ChildStdin,
    receiver: mpsc::Receiver<Message>,
    input: Sender<ServiceInput>,
) {
    let mut writer = BufWriter::new(stdin);
    for message in receiver {
        if let Err(error) = framing::write_message(&mut writer, &message.to_value()) {
            send_input(
                &input,
                ClientEvent::WriterClosed {
                    generation,
                    error: error.to_string(),
                },
            );
            return;
        }
    }
}

fn read_loop(generation: u64, stdout: std::process::ChildStdout, input: Sender<ServiceInput>) {
    let mut reader = BufReader::new(stdout);
    loop {
        match framing::read_message(&mut reader) {
            Ok(Some(value)) => match Message::parse(value) {
                Ok(message) => send_input(
                    &input,
                    ClientEvent::Message {
                        generation,
                        message,
                    },
                ),
                Err(error) => {
                    send_input(
                        &input,
                        ClientEvent::ReaderClosed {
                            generation,
                            error: Some(error.to_string()),
                        },
                    );
                    return;
                }
            },
            Ok(None) => {
                send_input(
                    &input,
                    ClientEvent::ReaderClosed {
                        generation,
                        error: None,
                    },
                );
                return;
            }
            Err(error) => {
                send_input(
                    &input,
                    ClientEvent::ReaderClosed {
                        generation,
                        error: Some(error.to_string()),
                    },
                );
                return;
            }
        }
    }
}

fn stderr_loop(generation: u64, stderr: std::process::ChildStderr, input: Sender<ServiceInput>) {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return,
            Ok(_) => {
                let text = line.trim_end_matches(['\r', '\n']);
                let text = truncate_log(text);
                send_input(
                    &input,
                    ClientEvent::Stderr {
                        generation,
                        line: text,
                    },
                );
            }
            Err(_) => return,
        }
    }
}

fn truncate_log(line: &str) -> String {
    if line.len() <= MAX_LOG_LINE {
        return line.to_string();
    }
    let mut end = MAX_LOG_LINE;
    while !line.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    line[..end].to_string()
}

fn send_input(input: &Sender<ServiceInput>, event: ClientEvent) {
    let _ = input.send(ServiceInput::Client(event));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_long_log_lines_at_a_character_boundary() {
        let line = "𝄞".repeat(MAX_LOG_LINE);
        let truncated = truncate_log(&line);
        assert!(truncated.len() <= MAX_LOG_LINE);
        assert!(truncated.is_char_boundary(truncated.len()));
    }
}
