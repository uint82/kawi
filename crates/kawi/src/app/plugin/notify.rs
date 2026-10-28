use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use kawi_editor::BufferInfo;
use kawi_plugin::{
    ActivationEvent, BufferClosedEvent, BufferOpenedEvent, FileOpenedEvent, ModeChangedEvent,
    NotifyOutcome,
};
use serde_json::Value;

use super::PluginRuntime;
use crate::app::path_match::glob_matches;
use crate::keymap::KeyContext;

const MAX_ACTIVATION_EVENTS: usize = 256;

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
        let opened = serde_json::to_value(BufferOpenedEvent {
            buffer: number,
            path: info.path.clone(),
            language: info.language.clone(),
            revision: info.revision,
        })
        .unwrap_or(Value::Null);
        self.notify_event("buffer_opened", opened.clone());
        let Some(path) = info.path.as_deref() else {
            return;
        };
        let file_opened = serde_json::to_value(FileOpenedEvent {
            buffer: number,
            path: path.to_string(),
            language: info.language.clone(),
            revision: info.revision,
        })
        .unwrap_or(Value::Null);
        self.notify_event("file_opened", file_opened.clone());
        self.activate_file_open(
            path,
            info.language.as_deref(),
            &[("buffer_opened", opened), ("file_opened", file_opened)],
        );
    }

    fn activate_file_open(&mut self, path: &str, language: Option<&str>, events: &[(&str, Value)]) {
        let Some(host) = self.host.as_ref() else {
            return;
        };
        let active = self
            .catalog
            .plugins
            .iter()
            .filter(|plugin| {
                plugin
                    .manifest
                    .activation_events
                    .iter()
                    .any(|event| activation_matches(event, path, language))
            })
            .map(|plugin| {
                (
                    plugin.manifest.name.clone(),
                    plugin.manifest.subscribes.clone(),
                )
            })
            .collect::<Vec<_>>();
        for (name, subscribes) in active {
            if self.status(&name) == Some(kawi_plugin::PluginStatus::Ready) {
                continue;
            }
            let _ = host.activate(&name);
            let pending = self.pending_activation_events.entry(name).or_default();
            for (event, payload) in events {
                if !subscribes.iter().any(|subscribed| subscribed == event) {
                    continue;
                }
                if pending.len() >= MAX_ACTIVATION_EVENTS {
                    pending.pop_front();
                }
                pending.push_back(((*event).to_string(), payload.clone()));
            }
        }
    }

    pub(super) fn flush_activation_events(&mut self) {
        let ready = self
            .pending_activation_events
            .keys()
            .filter(|name| self.status(name) == Some(kawi_plugin::PluginStatus::Ready))
            .cloned()
            .collect::<Vec<_>>();
        for name in ready {
            let Some(events) = self.pending_activation_events.remove(&name) else {
                continue;
            };
            for (event, payload) in events {
                if self.send(&name, &event, payload) == Some(NotifyOutcome::Overflow) {
                    self.push_log(&name, "event queue full; dropping activation event");
                }
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

    pub(super) fn send(&self, plugin: &str, event: &str, payload: Value) -> Option<NotifyOutcome> {
        self.host.as_ref()?.notify(plugin, event, payload).ok()
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
