use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use kawi_process::{signal_group, ProcessSignal};
use portable_pty::{Child, ChildKiller, ExitStatus};

use super::wake::Wake;

const HANGUP_GRACE: Duration = Duration::from_millis(200);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

pub(super) enum ExitEvent {
    Status(ExitStatus),
    Error(String),
}

pub(super) struct ChildProcess {
    process_id: Option<u32>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    exit: Receiver<ExitEvent>,
    exited: Arc<AtomicBool>,
    waiter: Option<JoinHandle<()>>,
}

impl ChildProcess {
    pub(super) fn start(child: Box<dyn Child + Send + Sync>, wake: Wake) -> io::Result<Self> {
        let process_id = child.process_id();
        let killer = child.clone_killer();
        let (sender, exit) = mpsc::channel();
        let exited = Arc::new(AtomicBool::new(false));
        let thread_exited = Arc::clone(&exited);
        let waiter = thread::Builder::new()
            .name("kawi-terminal-wait".to_string())
            .spawn(move || wait_for_child(child, sender, wake, thread_exited));
        match waiter {
            Ok(waiter) => Ok(Self {
                process_id,
                killer,
                exit,
                exited,
                waiter: Some(waiter),
            }),
            Err(error) => {
                let mut killer = killer;
                let _ = killer.kill();
                Err(error)
            }
        }
    }

    pub(super) fn process_id(&self) -> Option<u32> {
        self.process_id
    }

    pub(super) fn try_exit(&mut self) -> Option<ExitEvent> {
        self.exit.try_recv().ok()
    }

    pub(super) fn kill_group(&self) {
        if let Some(pid) = self.process_id {
            signal_group(pid, ProcessSignal::Kill);
        }
    }

    pub(super) fn terminate(&mut self) {
        if !self.exited.load(Ordering::SeqCst) {
            if let Some(pid) = self.process_id {
                signal_group(pid, ProcessSignal::Hangup);
            }
            if !wait_for_exit(&self.exited, HANGUP_GRACE) {
                self.kill_group();
                let _ = self.killer.kill();
            }
        }
        self.join_waiter();
    }

    pub(super) fn terminate_now(&mut self) {
        if !self.exited.load(Ordering::SeqCst) {
            self.kill_group();
            let _ = self.killer.kill();
        }
        self.join_waiter();
    }

    fn join_waiter(&mut self) {
        if let Some(waiter) = self.waiter.take() {
            let _ = waiter.join();
        }
    }
}

impl Drop for ChildProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

pub(super) fn kill_now(child: &mut (dyn Child + Send + Sync)) {
    if let Some(pid) = child.process_id() {
        signal_group(pid, ProcessSignal::Kill);
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn wait_for_child(
    mut child: Box<dyn Child + Send + Sync>,
    sender: Sender<ExitEvent>,
    wake: Wake,
    exited: Arc<AtomicBool>,
) {
    let event = match child.wait() {
        Ok(status) => ExitEvent::Status(status),
        Err(error) => ExitEvent::Error(error.to_string()),
    };
    let _ = sender.send(event);
    exited.store(true, Ordering::SeqCst);
    wake.notify();
}

fn wait_for_exit(exited: &AtomicBool, grace: Duration) -> bool {
    let deadline = Instant::now() + grace;
    loop {
        if exited.load(Ordering::SeqCst) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(POLL_INTERVAL);
    }
}
