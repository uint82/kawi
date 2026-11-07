use std::io;
use std::sync::mpsc::Sender;

use anyhow::Result;
use portable_pty::ExitStatus;

use crate::responder::TerminalResponder;
use crate::screen::TerminalScreen;

mod child;
mod config;
mod pty;
mod reader;
mod response;
mod wake;

use child::ExitEvent;
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
    exit_error: Option<String>,
    closed: bool,
}

impl TerminalSession {
    pub fn start(options: &SpawnOptions, wake: Sender<()>) -> Result<Self> {
        let pty = PtySession::start(options, wake)?;
        Ok(Self {
            pty,
            screen: TerminalScreen::new(options.rows, options.cols, SCROLLBACK_LINES),
            responder: TerminalResponder::default(),
            exit: None,
            exit_error: None,
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
        if let Some(event) = self.pty.try_exit() {
            self.record_exit(event);
            changed = true;
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
        if let Some(status) = self.exit.as_ref() {
            return Some(exit_description(status));
        }
        self.exit_error
            .as_ref()
            .map(|error| format!("process wait failed: {error}"))
    }

    pub fn is_running(&self) -> bool {
        self.exit.is_none() && self.exit_error.is_none()
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn is_complete(&self) -> bool {
        !self.is_running() && self.closed
    }

    pub fn terminate(&mut self) {
        self.pty.terminate();
        self.drain();
    }

    pub fn terminate_now(&mut self) {
        self.pty.terminate_now();
        self.drain();
    }

    fn record_exit(&mut self, event: ExitEvent) {
        match event {
            ExitEvent::Status(status) => self.exit = Some(status),
            ExitEvent::Error(error) => self.exit_error = Some(error),
        }
    }
}

#[cfg(test)]
mod tests;
