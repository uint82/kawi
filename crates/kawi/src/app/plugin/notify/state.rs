use kawi_plugin::{
    BufferEnterEvent, BufferLeaveEvent, CursorMovedEvent, FocusChangedEvent, SelectionChangedEvent,
    WindowEnterEvent, WindowLeaveEvent,
};
use serde_json::Value;

use super::super::PluginRuntime;

impl PluginRuntime {
    pub(super) fn sync_editor_events(
        &mut self,
        editor: &kawi_editor::Editor,
        windows: &crate::app::window::Windows,
        focus: Option<(u64, u64)>,
    ) {
        self.sync_active_buffer(editor);
        self.sync_active_window(windows);
        self.sync_cursor_and_selection(editor);
        self.sync_focus(focus);
    }

    fn sync_active_buffer(&mut self, editor: &kawi_editor::Editor) {
        let buffer = editor.buffer_number() as u64;
        let previous = self.last_active_buffer.replace(buffer);
        if previous.is_none() || previous == Some(buffer) {
            return;
        }
        if let Some(previous) = previous {
            let path = self
                .known_buffers
                .get(&previous)
                .and_then(|info| info.path.clone());
            self.notify_event(
                "buffer_leave",
                serde_json::to_value(BufferLeaveEvent {
                    buffer: previous,
                    path,
                })
                .unwrap_or(Value::Null),
            );
        }
        let Some(info) = editor
            .buffer_infos()
            .into_iter()
            .find(|info| info.number as u64 == buffer)
        else {
            return;
        };
        self.notify_event(
            "buffer_enter",
            serde_json::to_value(BufferEnterEvent {
                buffer,
                path: info.path,
                language: info.language,
                revision: info.revision,
            })
            .unwrap_or(Value::Null),
        );
    }

    fn sync_active_window(&mut self, windows: &crate::app::window::Windows) {
        let window = windows.active();
        let buffer = windows
            .view(window)
            .map(|view| view.buffer as u64)
            .unwrap_or_default();
        let current = (window, buffer);
        let previous = self.last_active_window.replace(current);
        if previous.is_none() || previous == Some(current) {
            return;
        }
        if let Some((window, buffer)) = previous {
            self.notify_event(
                "window_leave",
                serde_json::to_value(WindowLeaveEvent { window, buffer }).unwrap_or(Value::Null),
            );
        }
        self.notify_event(
            "window_enter",
            serde_json::to_value(WindowEnterEvent { window, buffer }).unwrap_or(Value::Null),
        );
    }

    fn sync_cursor_and_selection(&mut self, editor: &kawi_editor::Editor) {
        let buffer = editor.buffer_number() as u64;
        let snapshot = editor.document_snapshot();
        let cursor = super::super::host_methods::position_at(&snapshot, editor.row(), editor.col());
        let selection =
            editor
                .selection_range()
                .map(
                    |((start_row, start_col), (end_row, end_col))| kawi_plugin::Selection {
                        start: super::super::host_methods::position_at(
                            &snapshot, start_row, start_col,
                        ),
                        end: super::super::host_methods::position_at(&snapshot, end_row, end_col),
                    },
                );
        let cursor_key = (buffer, cursor.clone());
        let previous_cursor = self.last_cursor.replace(cursor_key.clone());
        if previous_cursor.is_some() && previous_cursor != Some(cursor_key) {
            self.notify_event(
                "cursor_moved",
                serde_json::to_value(CursorMovedEvent {
                    buffer,
                    cursor,
                    selection: selection.clone(),
                })
                .unwrap_or(Value::Null),
            );
        }
        let selection_key = (buffer, selection.clone());
        let previous_selection = self.last_selection.replace(selection_key.clone());
        if previous_selection.is_some() && previous_selection != Some(selection_key) {
            self.notify_event(
                "selection_changed",
                serde_json::to_value(SelectionChangedEvent { buffer, selection })
                    .unwrap_or(Value::Null),
            );
        }
    }

    fn sync_focus(&mut self, focus: Option<(u64, u64)>) {
        if self.last_focus.is_none() || self.last_focus == Some(focus) {
            self.last_focus = Some(focus);
            return;
        }
        let previous = self.last_focus.replace(focus);
        if let Some(Some((window, buffer))) = previous {
            self.notify_event(
                "focus_lost",
                serde_json::to_value(FocusChangedEvent { window, buffer }).unwrap_or(Value::Null),
            );
        }
        if let Some((window, buffer)) = focus {
            self.notify_event(
                "focus_gained",
                serde_json::to_value(FocusChangedEvent { window, buffer }).unwrap_or(Value::Null),
            );
        }
    }
}
