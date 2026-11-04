use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, Child, MasterPty, PtyPair};

use super::child::{self, ChildProcess};
use super::config::{build_command, pty_size};
use super::reader;
use super::wake::Wake;
use super::PtyEvent;

const READ_QUEUE: usize = 256;

pub(super) struct PtySession {
    events: Option<Receiver<PtyEvent>>,
    wake: Wake,
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    child: ChildProcess,
    reader: Option<JoinHandle<()>>,
}

impl PtySession {
    pub(super) fn start(options: &super::SpawnOptions, wake: Sender<()>) -> Result<Self> {
        let pty_system = native_pty_system();
        let PtyPair { slave, master } = pty_system
            .openpty(pty_size(options.rows, options.cols))
            .context("open terminal pty")?;
        let command = build_command(options);
        let mut child = slave
            .spawn_command(command)
            .with_context(|| format!("start {}", options.shell))?;
        drop(slave);
        let (reader, writer) = open_endpoints(master.as_ref(), child.as_mut())?;
        let (sender, events) = mpsc::sync_channel(READ_QUEUE);
        let wake = Wake::new(wake);
        let reader = start_reader(reader, sender, child.as_mut(), wake.clone())?;
        let child = ChildProcess::start(child, wake.clone()).context("spawn terminal waiter")?;
        Ok(Self {
            events: Some(events),
            wake,
            master: Some(master),
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

    pub(super) fn try_exit(&mut self) -> Option<child::ExitEvent> {
        self.child.try_exit()
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

    pub(super) fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    pub(super) fn terminate(&mut self) {
        self.child.terminate();
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.terminate();
        self.writer.take();
        self.master.take();
        self.events.take();
        self.stop_reader();
    }
}

impl PtySession {
    fn stop_reader(&mut self) {
        if self
            .reader
            .as_ref()
            .is_some_and(|reader| !reader.is_finished())
        {
            self.child.kill_group();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn open_endpoints(
    master: &dyn MasterPty,
    child: &mut (dyn Child + Send + Sync),
) -> Result<(Box<dyn Read + Send>, Box<dyn Write + Send>)> {
    let reader = master
        .try_clone_reader()
        .inspect_err(|_| child::kill_now(child))
        .context("open terminal reader")?;
    let writer = master
        .take_writer()
        .inspect_err(|_| child::kill_now(child))
        .context("open terminal writer")?;
    Ok((reader, writer))
}

fn start_reader(
    reader: Box<dyn Read + Send>,
    sender: std::sync::mpsc::SyncSender<PtyEvent>,
    child: &mut (dyn Child + Send + Sync),
    wake: Wake,
) -> Result<JoinHandle<()>> {
    reader::spawn(reader, sender, wake)
        .inspect_err(|_| child::kill_now(child))
        .context("spawn terminal reader")
}
