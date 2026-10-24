mod app;
mod frame;
mod input;
mod scroll;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use anyhow::Result;
use kawi_terminal::{SpawnOptions, TerminalSession};

const MIN_HEIGHT: usize = 6;
const MAX_HEIGHT: usize = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalMode {
    Input,
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKey {
    Consumed,
    ClosePanel,
    FocusEditor,
}

pub struct TerminalUi {
    session: Option<TerminalSession>,
    mode: TerminalMode,
    open: bool,
    title: String,
    status: String,
    escape_pending: bool,
    one_shot: bool,
    normal_cursor: (u16, u16),
    exit_reported: bool,
    exit_notice: Option<String>,
}

impl Default for TerminalUi {
    fn default() -> Self {
        Self {
            session: None,
            mode: TerminalMode::Input,
            open: false,
            title: String::new(),
            status: String::new(),
            escape_pending: false,
            one_shot: false,
            normal_cursor: (0, 0),
            exit_reported: false,
            exit_notice: None,
        }
    }
}

impl TerminalUi {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn is_running(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(TerminalSession::is_running)
    }

    pub fn needs_poll(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.is_closed() && session.is_running())
    }

    pub fn take_exit(&mut self) -> Option<String> {
        self.exit_notice.take()
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn open(&mut self, options: SpawnOptions, wake: Sender<()>) -> Result<()> {
        self.close();
        let label = options
            .command
            .clone()
            .unwrap_or_else(|| shell_label(&options.shell));
        let title = format!("{}  {}", label, options.cwd.display());
        let session = TerminalSession::start(&options, wake)?;
        self.session = Some(session);
        self.open = true;
        self.mode = TerminalMode::Input;
        self.title = title;
        self.status = "running".to_string();
        self.escape_pending = false;
        self.one_shot = false;
        self.normal_cursor = (0, 0);
        self.exit_reported = false;
        self.exit_notice = None;
        Ok(())
    }

    pub fn focus_input(&mut self) {
        self.mode = TerminalMode::Input;
        self.escape_pending = false;
        self.one_shot = false;
        self.scroll_to_bottom();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.escape_pending = false;
        self.one_shot = false;
        self.exit_reported = false;
        self.exit_notice = None;
        self.session.take();
    }

    pub fn drain(&mut self) -> bool {
        let Some(session) = self.session.as_mut() else {
            return false;
        };
        let changed = session.drain();
        if !self.exit_reported {
            if let Some(status) = session.exit_description() {
                let cursor = session.screen().cursor();
                self.normal_cursor = (cursor.row, cursor.col);
                self.status = status.clone();
                self.exit_notice = Some(status);
                self.exit_reported = true;
                self.mode = TerminalMode::Normal;
                self.escape_pending = false;
                self.one_shot = false;
                return true;
            }
        }
        changed
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        let Some(session) = self.session.as_mut() else {
            return Ok(());
        };
        session.resize(rows, cols)
    }

    pub fn panel_height(&self, screen_height: usize, task_height: usize) -> usize {
        if !self.open {
            return 0;
        }
        Self::desired_panel_height(screen_height, task_height)
    }

    pub fn desired_panel_height(screen_height: usize, task_height: usize) -> usize {
        let available = screen_height.saturating_sub(1).saturating_sub(task_height);
        if available < MIN_HEIGHT {
            return 0;
        }
        (screen_height / 3)
            .clamp(MIN_HEIGHT, MAX_HEIGHT)
            .min(available)
    }
}

pub(super) fn spawn_options(
    cwd: PathBuf,
    rows: u16,
    cols: u16,
    command: Option<String>,
) -> SpawnOptions {
    // TODO: route EDITOR/GIT_EDITOR through a running-instance socket instead of a nested editor.
    let mut options = SpawnOptions::new(cwd, rows, cols);
    options.command = command;
    options.editor = std::env::current_exe().ok();
    options
}

fn shell_label(shell: &str) -> String {
    Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(shell)
        .to_string()
}
