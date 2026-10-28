use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use kawi_buffer::{BufferChange, BufferChangeOrigin, TextPosition};
use kawi_editor::BufferChanges;
use kawi_plugin::{
    plugin_origin, BufferChangedEvent, NotifyOutcome, Position, ProtocolTextEdit, ORIGIN_MIXED,
};
use serde_json::Value;

use super::PluginRuntime;

const MAX_PENDING_CHANGES: usize = 512;

struct ChangePayload {
    origin: String,
    full_resync: bool,
    changes: Vec<ProtocolTextEdit>,
}

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
                let own = buffer
                    .changes
                    .iter()
                    .any(|change| is_own_edit(change, name));
                let visible = buffer
                    .changes
                    .iter()
                    .filter(|change| !is_own_edit(change, name))
                    .collect::<Vec<_>>();
                if visible.is_empty() {
                    continue;
                }
                let origins = buffer
                    .changes
                    .iter()
                    .filter(|change| !is_own_edit(change, name))
                    .map(origin_of)
                    .collect::<BTreeSet<_>>();
                let mixed = own || origins.len() > 1;
                let origin = if mixed {
                    ORIGIN_MIXED.to_string()
                } else {
                    origins
                        .into_iter()
                        .next()
                        .unwrap_or_else(|| "user".to_string())
                };
                let changes = if mixed {
                    Vec::new()
                } else {
                    visible.into_iter().map(to_protocol_edit).collect()
                };
                self.queue_buffer_change(
                    name,
                    *debounce,
                    &buffer,
                    now,
                    ChangePayload {
                        origin,
                        full_resync: mixed,
                        changes,
                    },
                );
            }
        }
    }

    fn queue_buffer_change(
        &mut self,
        name: &str,
        debounce: u64,
        buffer: &BufferChanges,
        now: Instant,
        payload: ChangePayload,
    ) {
        let key = (name.to_string(), buffer.number as u64);
        let needs_resync = payload.full_resync || self.resync_buffers.contains(&key);
        match self.pending.get_mut(&key) {
            Some(pending) => {
                pending.revision = buffer.revision;
                pending.due = now + Duration::from_millis(debounce);
                if pending.origin != payload.origin {
                    pending.origin = ORIGIN_MIXED.to_string();
                    pending.full_resync = true;
                    pending.changes.clear();
                    return;
                }
                if pending.full_resync || needs_resync {
                    pending.full_resync = true;
                    pending.changes.clear();
                    return;
                }
                pending.changes.extend(payload.changes);
                if pending.changes.len() > MAX_PENDING_CHANGES {
                    pending.changes.clear();
                    pending.full_resync = true;
                }
            }
            None => {
                let previous = self.last_delivered.get(&key).copied();
                self.pending.insert(
                    key,
                    PendingBufferEvent {
                        revision: buffer.revision,
                        previous_revision: previous.unwrap_or(buffer.revision.saturating_sub(1)),
                        origin: payload.origin,
                        full_resync: previous.is_none() || needs_resync,
                        changes: if previous.is_none() || needs_resync {
                            Vec::new()
                        } else {
                            payload.changes
                        },
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
                    self.last_delivered.insert(key.clone(), pending.revision);
                    self.resync_buffers.remove(&key);
                }
                Some(NotifyOutcome::Overflow) => {
                    self.last_delivered.remove(&key);
                    self.resync_buffers.insert(key.clone());
                    self.push_log(&key.0, "event queue full; dropping buffer change");
                }
                _ => {
                    self.last_delivered.remove(&key);
                    self.resync_buffers.insert(key);
                }
            }
        }
    }

    pub(super) fn clear_plugin_state(&mut self, plugin: &str) {
        self.pending_activation_events.remove(plugin);
        self.pending.retain(|(name, _), _| name.as_str() != plugin);
        self.last_delivered
            .retain(|(name, _), _| name.as_str() != plugin);
        self.resync_buffers
            .retain(|(name, _)| name.as_str() != plugin);
    }
}

fn is_own_edit(change: &BufferChange, plugin: &str) -> bool {
    match &change.origin {
        BufferChangeOrigin::Plugin(name) => name == plugin,
        _ => false,
    }
}

fn origin_of(change: &BufferChange) -> String {
    match &change.origin {
        BufferChangeOrigin::User => "user".to_string(),
        BufferChangeOrigin::Undo => "undo".to_string(),
        BufferChangeOrigin::Plugin(name) => plugin_origin(name),
        BufferChangeOrigin::ExternalReload => "external_reload".to_string(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use kawi_buffer::{BufferChange, BufferChangeOrigin, TextPosition};
    use kawi_editor::BufferChanges;

    fn buffer(revision: u64) -> BufferChanges {
        BufferChanges {
            number: 1,
            path: None,
            language: None,
            revision,
            changes: vec![BufferChange {
                start: TextPosition {
                    line: 0,
                    byte_column: 0,
                },
                end: TextPosition {
                    line: 0,
                    byte_column: 0,
                },
                text: "x".to_string(),
                origin: BufferChangeOrigin::User,
            }],
        }
    }

    fn edit() -> ProtocolTextEdit {
        ProtocolTextEdit {
            start: Position {
                line: 0,
                byte_column: 0,
            },
            end: Position {
                line: 0,
                byte_column: 0,
            },
            text: "x".to_string(),
        }
    }

    #[test]
    fn coalescing_uses_the_last_delivered_revision() {
        let mut runtime = PluginRuntime::default();
        let key = ("events".to_string(), 1);
        runtime.last_delivered.insert(key.clone(), 3);
        runtime.queue_buffer_change(
            "events",
            0,
            &buffer(5),
            Instant::now(),
            ChangePayload {
                origin: "user".to_string(),
                full_resync: false,
                changes: vec![edit()],
            },
        );
        let pending = runtime.pending.get(&key).unwrap();
        assert_eq!(pending.previous_revision, 3);
        assert!(!pending.full_resync);
    }

    #[test]
    fn overflow_resync_clears_changes() {
        let mut runtime = PluginRuntime::default();
        let key = ("events".to_string(), 1);
        runtime.resync_buffers.insert(key.clone());
        runtime.queue_buffer_change(
            "events",
            0,
            &buffer(5),
            Instant::now(),
            ChangePayload {
                origin: "user".to_string(),
                full_resync: false,
                changes: vec![edit()],
            },
        );
        let pending = runtime.pending.get(&key).unwrap();
        assert!(pending.full_resync);
        assert!(pending.changes.is_empty());
    }

    #[test]
    fn clearing_a_crashed_plugin_drops_queued_events_and_forces_resync() {
        let mut runtime = PluginRuntime::default();
        let key = ("events".to_string(), 1);
        runtime.queue_buffer_change(
            "events",
            0,
            &buffer(5),
            Instant::now(),
            ChangePayload {
                origin: "user".to_string(),
                full_resync: false,
                changes: vec![edit()],
            },
        );
        runtime.last_delivered.insert(key.clone(), 5);
        runtime.clear_plugin_state("events");
        assert!(runtime.pending.is_empty());
        assert!(runtime.last_delivered.is_empty());
        runtime.queue_buffer_change(
            "events",
            0,
            &buffer(6),
            Instant::now(),
            ChangePayload {
                origin: "user".to_string(),
                full_resync: false,
                changes: vec![edit()],
            },
        );
        assert!(runtime.pending.get(&key).unwrap().full_resync);
    }

    #[test]
    fn coalescing_different_origins_forces_resync() {
        let mut runtime = PluginRuntime::default();
        let key = ("events".to_string(), 1);
        runtime.queue_buffer_change(
            "events",
            0,
            &buffer(5),
            Instant::now(),
            ChangePayload {
                origin: "user".to_string(),
                full_resync: false,
                changes: vec![edit()],
            },
        );
        runtime.queue_buffer_change(
            "events",
            0,
            &buffer(6),
            Instant::now(),
            ChangePayload {
                origin: "undo".to_string(),
                full_resync: false,
                changes: vec![edit()],
            },
        );
        let pending = runtime.pending.get(&key).unwrap();
        assert!(pending.full_resync);
        assert!(pending.changes.is_empty());
        assert_eq!(pending.origin, ORIGIN_MIXED);
    }
}
