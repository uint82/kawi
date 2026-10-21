use crate::framing;
use crate::process::ServerProcess;
use crate::protocol::Message;
use crate::service::ServiceInput;
use crate::ServerConfig;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

const MAX_LOG_LINE: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ClientKey {
    pub(crate) server: String,
    pub(crate) root: PathBuf,
}

#[derive(Debug)]
pub(crate) enum ClientEvent {
    Message {
        key: ClientKey,
        message: Message,
    },
    ReaderClosed {
        key: ClientKey,
        error: Option<String>,
    },
    WriterClosed {
        key: ClientKey,
        error: String,
    },
    Stderr {
        key: ClientKey,
        line: String,
    },
}

pub(crate) struct Client {
    process: ServerProcess,
    writer: Sender<Message>,
}

impl Client {
    pub(crate) fn spawn(
        key: ClientKey,
        config: &ServerConfig,
        input: Sender<ServiceInput>,
    ) -> Result<Self> {
        let (mut process, io) = ServerProcess::spawn(config, &key.root)?;
        let (writer, writer_rx) = mpsc::channel();
        if let Err(error) = spawn_writer(key.clone(), io.stdin, writer_rx, input.clone()) {
            process.terminate();
            return Err(error);
        }
        if let Err(error) = spawn_reader(key.clone(), io.stdout, input.clone()) {
            process.terminate();
            return Err(error);
        }
        if let Err(error) = spawn_stderr(key.clone(), io.stderr, input) {
            process.terminate();
            return Err(error);
        }
        Ok(Self { process, writer })
    }

    pub(crate) fn send(&self, message: Message) -> bool {
        self.writer.send(message).is_ok()
    }

    pub(crate) fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        self.process.try_wait()
    }

    pub(crate) fn wait_timeout(&mut self, timeout: Duration) -> Option<ExitStatus> {
        self.process.wait_timeout(timeout)
    }

    pub(crate) fn terminate(&mut self) {
        self.process.terminate();
    }
}

fn spawn_writer(
    key: ClientKey,
    stdin: std::process::ChildStdin,
    receiver: mpsc::Receiver<Message>,
    input: Sender<ServiceInput>,
) -> Result<()> {
    thread::Builder::new()
        .name(format!("lsp-write-{}", key.server))
        .spawn(move || write_loop(key, stdin, receiver, input))
        .context("spawn lsp writer")?;
    Ok(())
}

fn spawn_reader(
    key: ClientKey,
    stdout: std::process::ChildStdout,
    input: Sender<ServiceInput>,
) -> Result<()> {
    thread::Builder::new()
        .name(format!("lsp-read-{}", key.server))
        .spawn(move || read_loop(key, stdout, input))
        .context("spawn lsp reader")?;
    Ok(())
}

fn spawn_stderr(
    key: ClientKey,
    stderr: std::process::ChildStderr,
    input: Sender<ServiceInput>,
) -> Result<()> {
    thread::Builder::new()
        .name(format!("lsp-log-{}", key.server))
        .spawn(move || stderr_loop(key, stderr, input))
        .context("spawn lsp stderr reader")?;
    Ok(())
}

fn write_loop(
    key: ClientKey,
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
                    key,
                    error: error.to_string(),
                },
            );
            return;
        }
    }
}

fn read_loop(key: ClientKey, stdout: std::process::ChildStdout, input: Sender<ServiceInput>) {
    let mut reader = BufReader::new(stdout);
    loop {
        match framing::read_message(&mut reader) {
            Ok(Some(value)) => match Message::parse(value) {
                Ok(message) => send_input(
                    &input,
                    ClientEvent::Message {
                        key: key.clone(),
                        message,
                    },
                ),
                Err(error) => {
                    send_input(
                        &input,
                        ClientEvent::ReaderClosed {
                            key,
                            error: Some(error.to_string()),
                        },
                    );
                    return;
                }
            },
            Ok(None) => {
                send_input(&input, ClientEvent::ReaderClosed { key, error: None });
                return;
            }
            Err(error) => {
                send_input(
                    &input,
                    ClientEvent::ReaderClosed {
                        key,
                        error: Some(error.to_string()),
                    },
                );
                return;
            }
        }
    }
}

fn stderr_loop(key: ClientKey, stderr: std::process::ChildStderr, input: Sender<ServiceInput>) {
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
                        key: key.clone(),
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
