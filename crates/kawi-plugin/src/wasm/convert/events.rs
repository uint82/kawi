use serde_json::Value;

use crate::protocol::{
    BufferChangedEvent, BufferClosedEvent, BufferEnterEvent, BufferLeaveEvent, BufferOpenedEvent,
    CursorMovedEvent, DiagnosticSummary, DiagnosticsChangedEvent, FileOpenedEvent, FileSavedEvent,
    FocusChangedEvent, LanguageChangedEvent, ModeChangedEvent, ProtocolTextEdit, RpcError,
    SelectionChangedEvent, UserEvent, WindowClosedEvent, WindowCreatedEvent, WindowEnterEvent,
    WindowLeaveEvent,
};

use super::super::bindings::kawi::plugin::types::{
    BufferChanged, BufferClosed, BufferEnter, BufferLeave, BufferOpened, CursorMoved, Diagnostic,
    DiagnosticsChanged, Event, FileOpened, FileSaved, FocusChanged, LanguageChanged, ModeChanged,
    SelectionChanged, UserEvent as WitUserEvent, WindowEnter, WindowLeave, WindowLifecycle,
};

use super::plugin_control::selection_from_protocol;

pub fn event_from_json(event: &str, payload: Value) -> Result<Event, RpcError> {
    match event {
        "buffer_changed" => {
            let event: BufferChangedEvent = parse(payload)?;
            Ok(Event::BufferChanged(BufferChanged {
                buffer: event.buffer,
                revision: event.revision,
                previous_revision: event.previous_revision,
                origin: event.origin,
                full_resync: event.full_resync,
                changes: event
                    .changes
                    .into_iter()
                    .map(edit_from_protocol)
                    .collect::<Result<_, _>>()?,
            }))
        }
        "buffer_opened" => {
            let event: BufferOpenedEvent = parse(payload)?;
            Ok(Event::BufferOpened(BufferOpened {
                buffer: event.buffer,
                path: event.path,
                language: event.language,
                revision: event.revision,
            }))
        }
        "file_opened" => {
            let event: FileOpenedEvent = parse(payload)?;
            Ok(Event::FileOpened(FileOpened {
                buffer: event.buffer,
                path: event.path,
                language: event.language,
                revision: event.revision,
            }))
        }
        "buffer_closed" => {
            let event: BufferClosedEvent = parse(payload)?;
            Ok(Event::BufferClosed(BufferClosed {
                buffer: event.buffer,
                path: event.path,
            }))
        }
        "file_saved" => {
            let event: FileSavedEvent = parse(payload)?;
            Ok(Event::FileSaved(FileSaved {
                buffer: event.buffer,
                path: event.path,
                revision: event.revision,
            }))
        }
        "buffer_enter" => {
            let event: BufferEnterEvent = parse(payload)?;
            Ok(Event::BufferEnter(BufferEnter {
                buffer: event.buffer,
                path: event.path,
                language: event.language,
                revision: event.revision,
            }))
        }
        "buffer_leave" => {
            let event: BufferLeaveEvent = parse(payload)?;
            Ok(Event::BufferLeave(BufferLeave {
                buffer: event.buffer,
                path: event.path,
            }))
        }
        "window_enter" => {
            let event: WindowEnterEvent = parse(payload)?;
            Ok(Event::WindowEnter(WindowEnter {
                window: event.window,
                buffer: event.buffer,
            }))
        }
        "window_leave" => {
            let event: WindowLeaveEvent = parse(payload)?;
            Ok(Event::WindowLeave(WindowLeave {
                window: event.window,
                buffer: event.buffer,
            }))
        }
        "cursor_moved" => {
            let event: CursorMovedEvent = parse(payload)?;
            Ok(Event::CursorMoved(CursorMoved {
                buffer: event.buffer,
                cursor: super::position_from_protocol(event.cursor)?,
                selection: event.selection.map(selection_from_protocol).transpose()?,
            }))
        }
        "selection_changed" => {
            let event: SelectionChangedEvent = parse(payload)?;
            Ok(Event::SelectionChanged(SelectionChanged {
                buffer: event.buffer,
                selection: event.selection.map(selection_from_protocol).transpose()?,
            }))
        }
        "focus_gained" => {
            let event: FocusChangedEvent = parse(payload)?;
            Ok(Event::FocusGained(FocusChanged {
                window: event.window,
                buffer: event.buffer,
            }))
        }
        "focus_lost" => {
            let event: FocusChangedEvent = parse(payload)?;
            Ok(Event::FocusLost(FocusChanged {
                window: event.window,
                buffer: event.buffer,
            }))
        }
        "mode_changed" => {
            let event: ModeChangedEvent = parse(payload)?;
            Ok(Event::ModeChanged(ModeChanged {
                buffer: event.buffer,
                previous: event.from,
                next: event.to,
            }))
        }
        "diagnostics_changed" => {
            let event: DiagnosticsChangedEvent = parse(payload)?;
            Ok(Event::DiagnosticsChanged(DiagnosticsChanged {
                buffer: event.buffer,
                path: event.path,
                revision: event.revision,
                diagnostics: event
                    .diagnostics
                    .into_iter()
                    .map(diagnostic_from_protocol)
                    .collect(),
            }))
        }
        _ => Err(RpcError::invalid_params(format!(
            "unknown plugin event: {event}"
        ))),
    }
}

fn parse<T: serde::de::DeserializeOwned>(payload: Value) -> Result<T, RpcError> {
    serde_json::from_value(payload).map_err(|error| RpcError::invalid_params(error.to_string()))
}

fn edit_from_protocol(
    edit: ProtocolTextEdit,
) -> Result<super::super::bindings::kawi::plugin::types::Edit, RpcError> {
    Ok(super::super::bindings::kawi::plugin::types::Edit {
        start: super::position_from_protocol(edit.start)?,
        end: super::position_from_protocol(edit.end)?,
        text: edit.text,
    })
}

fn diagnostic_from_protocol(diagnostic: DiagnosticSummary) -> Diagnostic {
    Diagnostic {
        start: super::position_from_protocol_lossy(diagnostic.start),
        end: super::position_from_protocol_lossy(diagnostic.end),
        severity: super::severity_from_name(&diagnostic.severity),
        message: diagnostic.message,
        source: diagnostic.source,
        code: diagnostic.code,
    }
}

pub fn callback_event_from_json(
    event: &str,
    payload: Value,
) -> Result<super::super::bindings::CallbackEvent, RpcError> {
    match event {
        "window_created" => {
            let event: WindowCreatedEvent = parse(payload)?;
            Ok(super::super::bindings::CallbackEvent::WindowCreated(
                WindowLifecycle {
                    window: event.window,
                    buffer: event.buffer,
                },
            ))
        }
        "window_closed" => {
            let event: WindowClosedEvent = parse(payload)?;
            Ok(super::super::bindings::CallbackEvent::WindowClosed(
                WindowLifecycle {
                    window: event.window,
                    buffer: event.buffer,
                },
            ))
        }
        "language_changed" => {
            let event: LanguageChangedEvent = parse(payload)?;
            Ok(super::super::bindings::CallbackEvent::LanguageChanged(
                LanguageChanged {
                    buffer: event.buffer,
                    path: event.path,
                    previous: event.previous,
                    language: event.language,
                },
            ))
        }
        "user" => {
            let event: UserEvent = parse(payload)?;
            Ok(super::super::bindings::CallbackEvent::User(WitUserEvent {
                name: event.name,
                payload: event.payload,
            }))
        }
        _ => Ok(super::super::bindings::CallbackEvent::Legacy(
            event_from_json(event, payload)?,
        )),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn maps_window_callbacks() {
        let created = callback_event_from_json("window_created", json!({"window": 1, "buffer": 2}))
            .expect("window_created");
        assert!(matches!(
            created,
            super::super::super::bindings::CallbackEvent::WindowCreated(WindowLifecycle {
                window: 1,
                buffer: 2,
            })
        ));
        let closed = callback_event_from_json("window_closed", json!({"window": 3, "buffer": 4}))
            .expect("window_closed");
        assert!(matches!(
            closed,
            super::super::super::bindings::CallbackEvent::WindowClosed(WindowLifecycle {
                window: 3,
                buffer: 4,
            })
        ));
    }

    #[test]
    fn maps_language_and_user_callbacks() {
        let language = callback_event_from_json(
            "language_changed",
            json!({"buffer": 1, "path": "src/main.rs", "previous": null, "language": "rust"}),
        )
        .expect("language_changed");
        assert!(matches!(
            language,
            super::super::super::bindings::CallbackEvent::LanguageChanged(LanguageChanged {
                buffer: 1,
                ref path,
                previous: None,
                ref language,
            }) if path.as_deref() == Some("src/main.rs") && language.as_deref() == Some("rust")
        ));
        let user = callback_event_from_json("user", json!({"name": "custom", "payload": "data"}))
            .expect("user");
        assert!(matches!(
            user,
            super::super::super::bindings::CallbackEvent::User(WitUserEvent {
                ref name,
                payload: Some(ref payload),
            }) if name == "custom" && payload == "data"
        ));
    }

    #[test]
    fn maps_known_events_to_legacy_callbacks() {
        let callback = callback_event_from_json(
            "file_saved",
            json!({"buffer": 1, "path": "src/main.rs", "revision": 2}),
        )
        .expect("file_saved");
        assert!(matches!(
            callback,
            super::super::super::bindings::CallbackEvent::Legacy(Event::FileSaved(FileSaved {
                buffer: 1,
                revision: 2,
                ..
            }))
        ));
    }

    #[test]
    fn rejects_unknown_callbacks() {
        assert!(callback_event_from_json("unknown", Value::Null).is_err());
    }
}
