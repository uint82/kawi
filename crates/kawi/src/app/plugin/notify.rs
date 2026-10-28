use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use kawi_buffer::{BufferChange, BufferChangeOrigin, TextPosition};
use kawi_editor::{BufferChanges, BufferInfo};
use kawi_plugin::{
    plugin_origin, ActivationEvent, BufferChangedEvent, BufferClosedEvent, BufferOpenedEvent,
    FileOpenedEvent, ModeChangedEvent, NotifyOutcome, Position, ProtocolTextEdit,
};
use serde_json::Value;

use super::PluginRuntime;
use crate::app::path_match::glob_matches;
use crate::keymap::KeyContext;

const MAX_PENDING_CHANGES: usize = 512;

pub(super) struct PendingBufferEvent {
    revision: u64,
    previous_revision: u64,
    origin: String,
    full_resync: bool,
    changes: Vec<ProtocolTextEdit>,
    due: Instant,
}

impl PluginRuntime {
    pub(super) fn sync_plugin_state(
        &mut self,
        editor: &mut kawi_editor::Editor,
        problems: &crate::app::problems::Problems,
    ) {
        self.sync_open_buffers(editor);
        self.sync_mode(editor);
        self.sync_diagnostics(editor, problems);
        let changes = editor.take_buffer_changes();
        self.publish_buffer_changes(changes);
        self.flush_pending(Instant::now());
    }

    fn sync_diagnostics(
        &mut self,
        editor: &kawi_editor::Editor,
        problems: &crate::app::problems::Problems,
    ) {
        if self.last_diagnostics_revision == problems.revision() {
            return;
        }
        self.last_diagnostics_revision = problems.revision();
        for info in editor.buffer_infos() {
            let Some(path) = info.path else {
                continue;
            };
            let Some(snapshot) = editor.buffer_snapshot(info.number) else {
                continue;
            };
            let entries = problems.problems_for_path(Path::new(&path));
            let diagnostics = super::diagnostics::summaries(&entries, &snapshot);
            self.notify_event(
                "diagnostics_changed",
                serde_json::to_value(kawi_plugin::DiagnosticsChangedEvent {
                    buffer: Some(info.number as u64),
                    path,
                    revision: problems.revision(),
                    diagnostics,
                })
                .unwrap_or(Value::Null),
            );
        }
    }

    pub(crate) fn file_saved(&mut self, buffer: usize, path: Option<String>, revision: u64) {
        self.notify_event(
            "file_saved",
            serde_json::to_value(kawi_plugin::FileSavedEvent {
                buffer: buffer as u64,
                path,
                revision,
            })
            .unwrap_or(Value::Null),
        );
    }

    fn sync_open_buffers(&mut self, editor: &kawi_editor::Editor) {
        let infos = editor.buffer_infos();
        let mut current = BTreeMap::new();
        for info in infos {
            let number = info.number as u64;
            if !self.known_buffers.contains_key(&number) {
                self.publish_buffer_opened(&info);
            }
            current.insert(number, info);
        }
        let closed = self
            .known_buffers
            .iter()
            .filter(|(number, _)| !current.contains_key(number))
            .map(|(number, info)| (*number, info.path.clone()))
            .collect::<Vec<_>>();
        for (number, path) in closed {
            self.notify_event(
                "buffer_closed",
                serde_json::to_value(BufferClosedEvent {
                    buffer: number,
                    path,
                })
                .unwrap_or(Value::Null),
            );
        }
        self.known_buffers = current;
    }

    fn publish_buffer_opened(&mut self, info: &BufferInfo) {
        let number = info.number as u64;
        self.notify_event(
            "buffer_opened",
            serde_json::to_value(BufferOpenedEvent {
                buffer: number,
                path: info.path.clone(),
                language: info.language.clone(),
                revision: info.revision,
            })
            .unwrap_or(Value::Null),
        );
        let Some(path) = info.path.as_deref() else {
            return;
        };
        self.notify_event(
            "file_opened",
            serde_json::to_value(FileOpenedEvent {
                buffer: number,
                path: path.to_string(),
                language: info.language.clone(),
                revision: info.revision,
            })
            .unwrap_or(Value::Null),
        );
        self.activate_file_open(path, info.language.as_deref());
    }

    fn activate_file_open(&self, path: &str, language: Option<&str>) {
        let Some(host) = self.host.as_ref() else {
            return;
        };
        for plugin in &self.catalog.plugins {
            if plugin
                .manifest
                .activation_events
                .iter()
                .any(|event| activation_matches(event, path, language))
            {
                let _ = host.activate(&plugin.manifest.name);
            }
        }
    }

    fn sync_mode(&mut self, editor: &kawi_editor::Editor) {
        let Some(context) = KeyContext::from_mode(editor.mode()) else {
            return;
        };
        let current = (editor.buffer_number() as u64, context.name().to_string());
        let previous = self.last_mode.replace(current.clone());
        let Some((buffer, from)) = previous else {
            return;
        };
        if buffer == current.0 && from == current.1 {
            return;
        }
        self.notify_event(
            "mode_changed",
            serde_json::to_value(ModeChangedEvent {
                buffer: current.0,
                from,
                to: current.1,
            })
            .unwrap_or(Value::Null),
        );
    }

    fn publish_buffer_changes(&mut self, buffers: Vec<BufferChanges>) {
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

    fn flush_pending(&mut self, now: Instant) {
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

    fn notify_event(&mut self, event: &str, payload: Value) {
        let mut overflowed = Vec::new();
        for plugin in &self.catalog.plugins {
            if self.send(&plugin.manifest.name, event, payload.clone())
                == Some(NotifyOutcome::Overflow)
            {
                overflowed.push(plugin.manifest.name.clone());
            }
        }
        for plugin in overflowed {
            self.push_log(&plugin, "event queue full; dropping event");
        }
    }

    fn send(&self, plugin: &str, event: &str, payload: Value) -> Option<NotifyOutcome> {
        self.host.as_ref()?.notify(plugin, event, payload).ok()
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

fn activation_matches(event: &ActivationEvent, path: &str, language: Option<&str>) -> bool {
    if event.event != "on_file_open" {
        return false;
    }
    let filetype_matches = event.filetypes.is_empty()
        || language.is_some_and(|language| {
            event
                .filetypes
                .iter()
                .any(|filetype| filetype.eq_ignore_ascii_case(language))
        });
    let glob_matches =
        event.globs.is_empty() || event.globs.iter().any(|glob| matches_path(glob, path));
    filetype_matches && glob_matches
}

fn matches_path(pattern: &str, path: &str) -> bool {
    let name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path);
    glob_matches(pattern, path) || glob_matches(pattern, name)
}
