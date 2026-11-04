use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use kawi_process::{signal_group, ProcessSignal};
use portable_pty::{native_pty_system, Child, ExitStatus, MasterPty};

use super::PtyEvent;
use super::config::{build_command, pty_size};
use super::reader;
use super::wake::Wake;

const READ_QUEUE: usize = 256;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const HANGUP_GRACE: Duration = Duration::from_millis(200);

pub(super) struct PtySession {
    events: Option<Receiver<PtyEvent>>,
    wake: Wake,
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    child: Box<dyn Child + Send + Sync>,
    reader: Option<JoinHandle<()>>,
}

impl PtySession {
    pub(super) fn start(options: &super::SpawnOptions, wake: Sender<()>) -> Result<Self> {
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
        let reader = pair
            .master
            .try_clone_reader()
            .inspect_err(|_| kill_child(child.as_mut()))
            .context("open terminal reader")?;
        let writer = pair
            .master
            .take_writer()
            .inspect_err(|_| kill_child(child.as_mut()))
            .context("open terminal writer")?;
        let (sender, events) = mpsc::sync_channel(READ_QUEUE);
        let wake = Wake::new(wake);
        let reader = reader::spawn(reader, sender, wake.clone())
            .inspect_err(|_| kill_child(child.as_mut()))
            .context("spawn terminal reader")?;
        Ok(Self {
            events: Some(events),
            wake,
            master: Some(pair.master),
            writer: Some(writer),
            child,
            reader: Some(reader),
        })
    }

    pub(super) fn drain_events(&mut self) -> Vec<PtyEvent> {
        self.wake.reset();
        let Some(receiver) = self.events.as_ref() else {
            return Vec::new();
        };
        let mut events = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            events.push(event);
        }
        events
    }

    pub(super) fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        let Some(writer) = self.writer.as_mut() else {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "terminal closed"));
        };
        writer.write_all(bytes)?;
        writer.flush()
    }

    pub(super) fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        let Some(master) = self.master.as_ref() else {
            return Ok(());
        };
        master
            .resize(pty_size(rows, cols))
            .context("resize terminal pty")
    }

    pub(super) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    pub(super) fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    pub(super) fn terminate(&mut self) {
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

fn kill_child(child: &mut (dyn Child + Send + Sync)) {
    let process_id = child.process_id();
    if let Some(pid) = process_id {
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
