use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Sender, SyncSender};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::capability::CapabilityRegistry;
use crate::error::PluginError;
use crate::host::{send_event, PluginHostEvent};

pub(super) type PendingMap = Arc<Mutex<HashMap<u64, Sender<Result<Value, PluginError>>>>>;

const MAX_LOG_LINE: usize = 8 * 1024;

pub(super) struct ReaderContext {
    pub(super) alive: Arc<AtomicBool>,
    pub(super) pending: PendingMap,
    pub(super) writer: SyncSender<Value>,
    pub(super) events: Sender<PluginHostEvent>,
    pub(super) wake: Sender<()>,
    pub(super) plugin: String,
}

pub(super) struct WriterContext {
    pub(super) alive: Arc<AtomicBool>,
    pub(super) pending: PendingMap,
    pub(super) events: Sender<PluginHostEvent>,
    pub(super) wake: Sender<()>,
    pub(super) plugin: String,
    pub(super) capabilities: CapabilityRegistry,
}

pub(super) fn remove_pending(pending: &PendingMap, id: u64) {
    pending
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&id);
}

pub(super) fn fail_pending(pending: &PendingMap, error: PluginError) {
    let waiting = std::mem::take(&mut *pending.lock().unwrap_or_else(|error| error.into_inner()));
    for (_, sender) in waiting {
        let _ = sender.send(Err(error.clone()));
    }
}

pub(super) fn truncate_log(text: &str) -> String {
    if text.len() <= MAX_LOG_LINE {
        return text.to_string();
    }
    let mut end = MAX_LOG_LINE;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

pub(super) fn send_log(
    events: &Sender<PluginHostEvent>,
    wake: &Sender<()>,
    plugin: &str,
    line: String,
) {
    send_event(
        events,
        wake,
        PluginHostEvent::Log {
            plugin: plugin.to_string(),
            line,
        },
    );
}
