use std::io::{BufRead, BufReader, BufWriter};
use std::process::{ChildStderr, ChildStdin};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

use serde_json::Value;

use super::channel::{fail_pending, send_log, truncate_log, WriterContext};
use crate::capability::is_event_allowed;
use crate::error::PluginError;
use crate::host::PluginHostEvent;

pub(super) fn spawn_writer(
    stdin: ChildStdin,
    receiver: Receiver<Value>,
    context: Arc<WriterContext>,
) -> Result<(), PluginError> {
    thread::Builder::new()
        .name("kawi-plugin-writer".to_string())
        .spawn(move || write_loop(stdin, receiver, context))
        .map(|_| ())
        .map_err(|error| PluginError::Spawn(error.to_string()))
}

fn write_loop(stdin: ChildStdin, receiver: Receiver<Value>, context: Arc<WriterContext>) {
    let mut writer = BufWriter::new(stdin);
    for message in receiver {
        if !event_is_allowed(&message, &context) {
            continue;
        }
        if let Err(error) = kawi_jsonrpc::framing::write_message(&mut writer, &message) {
            context.alive.store(false, Ordering::SeqCst);
            fail_pending(&context.pending, PluginError::ProcessExited);
            send_log(
                &context.events,
                &context.wake,
                &context.plugin,
                format!("write error: {error}"),
            );
            return;
        }
    }
}

fn event_is_allowed(message: &Value, context: &WriterContext) -> bool {
    if message.get("method").and_then(Value::as_str) != Some("event") {
        return true;
    }
    let Some(event) = message
        .get("params")
        .and_then(|params| params.get("event"))
        .and_then(Value::as_str)
    else {
        return true;
    };
    is_event_allowed(&context.capabilities, &context.plugin, event)
}

pub(super) fn spawn_stderr(
    stderr: ChildStderr,
    events: Sender<PluginHostEvent>,
    wake: Sender<()>,
    plugin: String,
) -> Result<(), PluginError> {
    thread::Builder::new()
        .name("kawi-plugin-stderr".to_string())
        .spawn(move || stderr_loop(stderr, events, wake, plugin))
        .map(|_| ())
        .map_err(|error| PluginError::Spawn(error.to_string()))
}

fn stderr_loop(
    stderr: ChildStderr,
    events: Sender<PluginHostEvent>,
    wake: Sender<()>,
    plugin: String,
) {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    while reader.read_line(&mut line).unwrap_or(0) > 0 {
        send_log(&events, &wake, &plugin, truncate_log(line.trim_end()));
        line.clear();
    }
}
