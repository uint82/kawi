use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct Wake {
    sender: Sender<()>,
    pending: Arc<AtomicBool>,
}

impl Wake {
    pub(super) fn new(sender: Sender<()>) -> Self {
        Self {
            sender,
            pending: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(super) fn notify(&self) {
        if !self.pending.swap(true, Ordering::SeqCst) {
            let _ = self.sender.send(());
        }
    }

    pub(super) fn reset(&self) {
        self.pending.store(false, Ordering::SeqCst);
    }
}
