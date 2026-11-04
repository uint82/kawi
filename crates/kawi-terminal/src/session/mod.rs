use std::io;
use anyhow::Result;
use portable_pty::ExitStatus;

use crate::responder::TerminalResponder;
use crate::screen::TerminalScreen;

mod config;
mod pty;
mod reader;
mod response;
mod wake;

pub use config::SpawnOptions;
use config::{exit_description, SCROLLBACK_LINES};
use pty::PtySession;

#[derive(Debug)]
pub enum PtyEvent {
    Data(Vec<u8>),
    Closed,
}

pub struct TerminalSession {
    pty: PtySession,
    screen: TerminalScreen,
    responder: TerminalResponder,
    exit: Option<ExitStatus>,
    closed: bool,
}

impl TerminalSession {
    pub fn start(options: &SpawnOptions, wake: std::sync::mpsc::Sender<()>) -> Result<Self> {
        let pty = PtySession::start(options, wake)?;
        Ok(Self {
            pty,
            screen: TerminalScreen::new(options.rows, options.cols, SCROLLBACK_LINES),
            responder: TerminalResponder::default(),
            exit: None,
            closed: false,
        })
    }

    pub fn process_id(&self) -> Option<u32> {
        self.pty.process_id()
    }

    pub fn drain(&mut self) -> bool {
        let mut changed = false;
        for event in self.pty.drain_events() {
            match event {
                PtyEvent::Data(bytes) => {
                    let queries = self.responder.process(&bytes);
                    self.screen.process(&bytes);
                    self.respond_to(queries);
                    changed = true;
                }
                PtyEvent::Closed => {
                    self.closed = true;
                    changed = true;
                }
            }
        }
        if self.exit.is_none() {
            if let Ok(Some(status)) = self.pty.try_wait() {
                self.exit = Some(status);
                changed = true;
            }
        }
        changed
    }

    pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.pty.write(bytes)
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.screen.resize(rows, cols);
        self.pty.resize(rows, cols)
    }

    pub fn screen(&self) -> &TerminalScreen {
        &self.screen
    }

    pub fn screen_mut(&mut self) -> &mut TerminalScreen {
        &mut self.screen
    }

    pub fn exit_status(&self) -> Option<&ExitStatus> {
        self.exit.as_ref()
    }

    pub fn exit_description(&self) -> Option<String> {
        self.exit.as_ref().map(exit_description)
    }

    pub fn is_running(&self) -> bool {
        self.exit.is_none()
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn is_complete(&self) -> bool {
        self.exit.is_some() && self.closed
    }

    pub fn needs_poll(&self) -> bool {
        !self.is_complete() && (self.closed || self.exit.is_some())
    }

    pub fn terminate(&mut self) {
        self.pty.terminate();
        self.exit = self.pty.try_wait().ok().flatten();
    }
}

#[cfg(test)]
mod tests;
