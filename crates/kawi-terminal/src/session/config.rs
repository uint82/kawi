use std::path::PathBuf;

use portable_pty::{CommandBuilder, ExitStatus, PtySize};

pub struct SpawnOptions {
    pub shell: String,
    pub cwd: PathBuf,
    pub rows: u16,
    pub cols: u16,
    pub command: Option<String>,
    pub editor: Option<String>,
    pub ipc_socket: Option<PathBuf>,
}

impl SpawnOptions {
    pub fn new(cwd: PathBuf, rows: u16, cols: u16) -> Self {
        Self {
            shell: default_shell(),
            cwd,
            rows,
            cols,
            command: None,
            editor: None,
            ipc_socket: None,
        }
    }
}

pub(super) fn build_command(options: &SpawnOptions) -> CommandBuilder {
    let mut builder = CommandBuilder::new(&options.shell);
    builder.cwd(&options.cwd);
    builder.env("TERM", "xterm-256color");
    builder.env("COLORTERM", "truecolor");
    builder.env("TERM_PROGRAM", "kawi");
    if let Some(editor) = options.editor.as_ref() {
        builder.env("EDITOR", editor);
        builder.env("GIT_EDITOR", editor);
    }
    match options.ipc_socket.as_ref() {
        Some(socket) => builder.env("KAWI_IPC_SOCKET", socket),
        None => builder.env_remove("KAWI_IPC_SOCKET"),
    };
    if let Some(command) = options.command.as_ref() {
        builder.arg("-c");
        builder.arg(command);
    }
    builder
}

pub(super) fn pty_size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}

pub(super) fn exit_description(status: &ExitStatus) -> String {
    match status.signal() {
        Some(signal) => format!("process exited: {signal}"),
        None => format!("process exited: {}", status.exit_code()),
    }
}

pub(super) const SCROLLBACK_LINES: usize = 10_000;

fn default_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string())
}
