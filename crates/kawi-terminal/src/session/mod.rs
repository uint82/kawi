use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use kawi_process::{signal_group, ProcessSignal};
use portable_pty::{native_pty_system, Child, ExitStatus, MasterPty};

use crate::responder::TerminalResponder;
use crate::screen::TerminalScreen;

mod config;
mod response;

pub use config::SpawnOptions;
use config::{build_command, exit_description, pty_size, SCROLLBACK_LINES};

const READ_BUFFER: usize = 8_192;
const READ_QUEUE: usize = 256;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const HANGUP_GRACE: Duration = Duration::from_millis(200);

#[derive(Debug)]
pub enum PtyEvent {
    Data(Vec<u8>),
    Closed,
}

pub struct PtySession {
    events: Option<Receiver<PtyEvent>>,
    wake_pending: Arc<AtomicBool>,
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    child: Box<dyn Child + Send + Sync>,
    reader: Option<JoinHandle<()>>,
}

impl PtySession {
    pub fn start(options: &SpawnOptions, wake: Sender<()>) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(pty_size(options.rows, options.cols))
            .context("open terminal pty")?;
        let command = build_command(options);
        let mut child = pair
            .slave
            .spawn_command(command)
            .with_context(|| format!("start {}", options.shell))?;
        drop(pair.slave);
        let reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(error) => {
                kill_child(child.as_mut());
                return Err(error).context("open terminal reader");
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(error) => {
                kill_child(child.as_mut());
                return Err(error).context("open terminal writer");
            }
        };
        let (sender, events) = mpsc::sync_channel(READ_QUEUE);
        let wake_pending = Arc::new(AtomicBool::new(false));
        let reader = match spawn_reader(reader, sender, wake, Arc::clone(&wake_pending)) {
            Ok(reader) => reader,
            Err(error) => {
                kill_child(child.as_mut());
                return Err(error).context("spawn terminal reader");
            }
        };
        Ok(Self {
            events: Some(events),
            wake_pending,
            master: Some(pair.master),
            writer: Some(writer),
            child,
            reader: Some(reader),
        })
    }

    pub fn drain_events(&mut self) -> Vec<PtyEvent> {
        self.wake_pending.store(false, Ordering::SeqCst);
        let Some(receiver) = self.events.as_ref() else {
            return Vec::new();
        };
        let mut events = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            events.push(event);
        }
        events
    }

    pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let Some(writer) = self.writer.as_mut() else {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "terminal closed"));
        };
        writer.write_all(bytes)?;
        writer.flush()
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        let Some(master) = self.master.as_ref() else {
            return Ok(());
        };
        master
            .resize(pty_size(rows, cols))
            .context("resize terminal pty")
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    pub fn terminate(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => {}
        }
        if let Some(pid) = self.child.process_id() {
            signal_group(pid, ProcessSignal::Hangup);
        }
        if wait_for_exit(self.child.as_mut(), HANGUP_GRACE) {
            return;
        }
        if let Some(pid) = self.child.process_id() {
            signal_group(pid, ProcessSignal::Kill);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.terminate();
        self.writer.take();
        self.master.take();
        self.events.take();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

pub struct TerminalSession {
    pty: PtySession,
    screen: TerminalScreen,
    responder: TerminalResponder,
    exit: Option<ExitStatus>,
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

fn spawn_reader(
    mut reader: Box<dyn Read + Send>,
    sender: SyncSender<PtyEvent>,
    wake: Sender<()>,
    wake_pending: Arc<AtomicBool>,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("kawi-terminal-reader".to_string())
        .spawn(move || {
            let mut buffer = [0u8; READ_BUFFER];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        if sender
                            .send(PtyEvent::Data(buffer[..read].to_vec()))
                            .is_err()
                        {
                            return;
                        }
                        wake_if_idle(&wake, &wake_pending);
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
            let _ = sender.send(PtyEvent::Closed);
            let _ = wake.send(());
        })
}

fn wake_if_idle(wake: &Sender<()>, wake_pending: &AtomicBool) {
    if !wake_pending.swap(true, Ordering::SeqCst) {
        let _ = wake.send(());
    }
}

fn kill_child(child: &mut (dyn Child + Send + Sync)) {
    if let Some(pid) = child.process_id() {
        signal_group(pid, ProcessSignal::Kill);
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn wait_for_exit(child: &mut (dyn Child + Send + Sync), grace: Duration) -> bool {
    let deadline = Instant::now() + grace;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
            Ok(None) | Err(_) => return false,
        }
    }
}

#[cfg(test)]
mod tests;
