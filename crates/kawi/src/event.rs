use anyhow::{Context, Result};
use crossterm::event::{read, Event};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(unix)]
use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGTERM};
#[cfg(unix)]
use signal_hook::iterator::Signals;

pub struct EventQueue {
    wake: Sender<()>,
    wake_rx: Receiver<()>,
    input: Arc<Mutex<VecDeque<Event>>>,
    input_closed: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
}

impl EventQueue {
    pub fn new() -> Self {
        let (wake, wake_rx) = mpsc::channel();
        Self {
            wake,
            wake_rx,
            input: Arc::new(Mutex::new(VecDeque::new())),
            input_closed: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn wake(&self) -> Sender<()> {
        self.wake.clone()
    }

    pub fn start_input_thread(&self) -> Result<()> {
        let input = Arc::clone(&self.input);
        let input_closed = Arc::clone(&self.input_closed);
        let wake = self.wake.clone();
        thread::Builder::new()
            .name("kawi-input".to_string())
            .spawn(move || read_input(input, wake, input_closed))
            .context("spawn input thread")?;
        Ok(())
    }

    pub fn start_signal_handlers(&self) -> Result<()> {
        #[cfg(unix)]
        {
            let mut signals =
                Signals::new([SIGHUP, SIGTERM, SIGINT]).context("register signal handlers")?;
            let wake = self.wake.clone();
            let shutdown = Arc::clone(&self.shutdown);
            thread::Builder::new()
                .name("kawi-signal".to_string())
                .spawn(move || {
                    for _ in signals.forever() {
                        shutdown.store(true, Ordering::SeqCst);
                        if wake.send(()).is_err() {
                            return;
                        }
                    }
                })
                .context("spawn signal thread")?;
        }
        Ok(())
    }

    pub fn take_input(&self) -> Vec<Event> {
        let mut input = self.input.lock().unwrap_or_else(|error| error.into_inner());
        input.drain(..).collect()
    }

    pub fn wait(&self) -> Result<()> {
        self.wake_rx.recv().context("wait for event")?;
        while self.wake_rx.try_recv().is_ok() {}
        Ok(())
    }

    pub fn input_closed(&self) -> bool {
        self.input_closed.load(Ordering::SeqCst)
    }

    pub fn shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub fn push_input(&self, event: Event) {
        self.input
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push_back(event);
    }
}

fn read_input(input: Arc<Mutex<VecDeque<Event>>>, wake: Sender<()>, input_closed: Arc<AtomicBool>) {
    loop {
        let event = match read() {
            Ok(event) => event,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => {
                input_closed.store(true, Ordering::SeqCst);
                let _ = wake.send(());
                return;
            }
        };
        input
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push_back(event);
        if wake.send(()).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn wake_wakes_the_main_loop() {
        let events = EventQueue::new();
        events.wake().send(()).unwrap();
        events.wait().unwrap();
    }

    #[test]
    fn shutdown_starts_clear_and_can_be_requested() {
        let events = EventQueue::new();
        assert!(!events.shutdown_requested());
        events.shutdown.store(true, Ordering::SeqCst);
        assert!(events.shutdown_requested());
    }

    #[test]
    fn input_is_drained_in_order() {
        let events = EventQueue::new();
        let first = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        let second = Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        events.push_input(first.clone());
        events.push_input(second.clone());
        assert_eq!(events.take_input(), vec![first, second]);
        assert!(events.take_input().is_empty());
    }
}
