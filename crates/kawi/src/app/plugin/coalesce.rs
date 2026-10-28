use std::time::{Duration, Instant};

use kawi_buffer::{BufferChange, BufferChangeOrigin, TextPosition};
use kawi_editor::BufferChanges;
use kawi_plugin::{plugin_origin, BufferChangedEvent, NotifyOutcome, Position, ProtocolTextEdit};
use serde_json::Value;

use super::PluginRuntime;

const MAX_PENDING_CHANGES: usize = 512;

pub(super) struct PendingBufferEvent {
    pub(super) revision: u64,
    pub(super) previous_revision: u64,
    pub(super) origin: String,
    pub(super) full_resync: bool,
    pub(super) changes: Vec<ProtocolTextEdit>,
    pub(super) due: Instant,
}

impl PluginRuntime {
    pub(super) fn publish_buffer_changes(&mut self, buffers: Vec<BufferChanges>) {
        let now = Instant::now();
        let subscribers = self
            .catalog
            .plugins
            .iter()
            .filter(|plugin| {
                plugin
                    .manifest
                    .subscribes
                    .iter()
                    .any(|event| event == "buffer_changed")
            })
            .map(|plugin| {
                (
                    plugin.manifest.name.clone(),
                    plugin
                        .manifest
                        .debounce
                        .get("buffer_changed")
                        .copied()
                        .unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();
        for buffer in buffers {
            for (name, debounce) in &subscribers {
                let changes = buffer
                    .changes
                    .iter()
                    .filter(|change| !is_own_edit(change, name))
                    .map(to_protocol_edit)
                    .collect::<Vec<_>>();
                if changes.is_empty() {
                    continue;
                }
                self.queue_buffer_change(name, *debounce, &buffer, changes, now);
            }
        }
    }

    fn queue_buffer_change(
        &mut self,
        name: &str,
        debounce: u64,
        buffer: &BufferChanges,
        changes: Vec<ProtocolTextEdit>,
        now: Instant,
    ) {
        let key = (name.to_string(), buffer.number as u64);
        match self.pending.get_mut(&key) {
            Some(pending) => {
                pending.revision = buffer.revision;
                pending.changes.extend(changes);
                pending.due = now + Duration::from_millis(debounce);
                if pending.changes.len() > MAX_PENDING_CHANGES {
                    pending.changes.clear();
                    pending.full_resync = true;
                }
            }
            None => {
                let previous = self.last_delivered.get(&key).copied();
                let origin = changes_origin(&buffer.changes);
                self.pending.insert(
                    key,
                    PendingBufferEvent {
                        revision: buffer.revision,
                        previous_revision: previous.unwrap_or(buffer.revision.saturating_sub(1)),
                        origin,
                        full_resync: previous.is_none(),
                        changes,
                        due: now + Duration::from_millis(debounce),
                    },
                );
            }
        }
    }

    pub(super) fn flush_pending(&mut self, now: Instant) {
        let due = self
            .pending
            .iter()
            .filter(|(_, pending)| pending.due <= now)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in due {
            let Some(pending) = self.pending.remove(&key) else {
                continue;
            };
            let payload = BufferChangedEvent {
                buffer: key.1,
                revision: pending.revision,
                previous_revision: pending.previous_revision,
                origin: pending.origin,
                full_resync: pending.full_resync,
                changes: pending.changes,
            };
            let outcome = self.send(
                &key.0,
                "buffer_changed",
                serde_json::to_value(payload).unwrap_or(Value::Null),
            );
            match outcome {
                Some(NotifyOutcome::Queued) => {
                    self.last_delivered.insert(key, pending.revision);
                }
                Some(NotifyOutcome::Overflow) => {
                    self.last_delivered.remove(&key);
                    self.push_log(&key.0, "event queue full; dropping buffer change");
                }
                _ => {
                    self.last_delivered.remove(&key);
                }
            }
        }
    }
}

fn is_own_edit(change: &BufferChange, plugin: &str) -> bool {
    match &change.origin {
        BufferChangeOrigin::Plugin(name) => name == plugin,
        _ => false,
    }
}

fn changes_origin(changes: &[BufferChange]) -> String {
    changes
        .last()
        .map(|change| match &change.origin {
            BufferChangeOrigin::User => "user".to_string(),
            BufferChangeOrigin::Undo => "undo".to_string(),
            BufferChangeOrigin::Plugin(name) => plugin_origin(name),
            BufferChangeOrigin::ExternalReload => "external_reload".to_string(),
        })
        .unwrap_or_else(|| "user".to_string())
}

fn to_protocol_edit(change: &BufferChange) -> ProtocolTextEdit {
    ProtocolTextEdit {
        start: to_position(change.start),
        end: to_position(change.end),
        text: change.text.clone(),
    }
}

fn to_position(position: TextPosition) -> Position {
    Position {
        line: position.line,
        byte_column: position.byte_column,
    }
}
