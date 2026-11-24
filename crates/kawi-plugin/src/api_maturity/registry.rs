use super::*;
use crate::capability::Capability;

const fn cap(capability: Capability) -> Option<Capability> {
    Some(capability)
}

const fn stable(since: u64) -> ApiMaturity {
    ApiMaturity::stable(since)
}

const fn experimental(since: u64) -> ApiMaturity {
    ApiMaturity::experimental(since)
}

macro_rules! host_methods {
    ($(($name:literal, $maturity:expr, $capability:expr, $interactive:literal, $mutates:literal)),+ $(,)?) => {
        pub(crate) const HOST_METHOD_ENTRIES: &[HostMethod] = &[
            $(HostMethod {
                name: $name,
                maturity: $maturity,
                capability: $capability,
                interactive: $interactive,
                mutates: $mutates,
            },)+
        ];
        pub(crate) const HOST_METHOD_NAMES: &[&str] = &[$($name,)+];
    };
}

macro_rules! events {
    ($(($name:literal, $maturity:expr, $capability:expr)),+ $(,)?) => {
        pub(crate) const EVENT_ENTRIES: &[Event] = &[
            $(Event {
                name: $name,
                maturity: $maturity,
                capability: $capability,
            },)+
        ];
        pub(crate) const EVENT_NAMES: &[&str] = &[$($name,)+];
    };
}

#[rustfmt::skip]
host_methods!(
    ("autocmd.clear", stable(WASM_AUTOCMDS_MINOR), cap(Capability::EditorAutocmds), false, false),
    ("autocmd.list", stable(WASM_AUTOCMDS_MINOR), cap(Capability::EditorAutocmds), false, false),
    ("autocmd.register", stable(WASM_AUTOCMDS_MINOR), cap(Capability::EditorAutocmds), false, false),
    ("autocmd.remove", stable(WASM_AUTOCMDS_MINOR), cap(Capability::EditorAutocmds), false, false),
    ("buffer.apply_edit", stable(WASM_EDITOR_MINOR), cap(Capability::EditorBuffersWrite), false, true),
    ("buffer.close", experimental(WASM_SCRATCH_BUFFERS_MINOR), cap(Capability::EditorBuffersWrite), false, true),
    ("buffer.create", experimental(WASM_SCRATCH_BUFFERS_MINOR), cap(Capability::EditorBuffersWrite), false, true),
    ("buffer.get", stable(WASM_EDITOR_MINOR), cap(Capability::EditorBuffers), false, false),
    ("buffer.list", stable(WASM_EDITOR_MINOR), cap(Capability::EditorBuffers), false, false),
    ("buffer.open", stable(WASM_EDITOR_MINOR), cap(Capability::EditorBuffersWrite), false, true),
    ("command.list", stable(WASM_EXTENSIONS_MINOR), cap(Capability::EditorCommands), false, false),
    ("command.register", stable(WASM_EXTENSIONS_MINOR), cap(Capability::EditorCommands), false, false),
    ("command.unregister", stable(WASM_EXTENSIONS_MINOR), cap(Capability::EditorCommands), false, false),
    ("config.get", stable(SETTINGS_MINOR), None, false, false),
    ("config.list", stable(SETTINGS_MINOR), None, false, false),
    ("editor.apply_edit", stable(LEGACY_MINOR), cap(Capability::EditorWrite), false, true),
    ("editor.emit_user_event", stable(WASM_AUTOCMDS_MINOR), cap(Capability::EditorAutocmds), false, false),
    ("editor.execute", stable(COMMAND_COMPOSITION_MINOR), cap(Capability::EditorExecute), false, true),
    ("editor.get_state", stable(LEGACY_MINOR), cap(Capability::EditorRead), false, false),
    ("editor.list_commands", stable(COMMAND_COMPOSITION_MINOR), cap(Capability::EditorExecute), false, false),
    ("editor.set_cursor", stable(WASM_EDITOR_MINOR), cap(Capability::EditorControl), false, true),
    ("editor.set_decorations", stable(DECORATIONS_MINOR), cap(Capability::EditorDecorations), false, true),
    ("editor.set_diagnostics", stable(DIAGNOSTICS_MINOR), cap(Capability::DiagnosticsPublish), false, true),
    ("editor.set_mode", stable(WASM_EDITOR_MINOR), cap(Capability::EditorControl), false, true),
    ("editor.set_selection", stable(WASM_EDITOR_MINOR), cap(Capability::EditorControl), false, true),
    ("keymap.list", stable(WASM_EXTENSIONS_MINOR), cap(Capability::EditorKeymaps), false, false),
    ("keymap.remove", stable(WASM_EXTENSIONS_MINOR), cap(Capability::EditorKeymaps), false, false),
    ("keymap.set", stable(WASM_EXTENSIONS_MINOR), cap(Capability::EditorKeymaps), false, false),
    ("options.get", stable(WASM_EDITOR_MINOR), cap(Capability::OptionsRead), false, false),
    ("options.list", stable(WASM_EDITOR_MINOR), cap(Capability::OptionsRead), false, false),
    ("options.reset", experimental(WASM_AUTOMATION_MINOR), cap(Capability::OptionsWrite), false, true),
    ("options.set", experimental(WASM_AUTOMATION_MINOR), cap(Capability::OptionsWrite), false, true),
    ("process.close_stdin", stable(PROCESS_JOBS_MINOR), cap(Capability::ProcessSpawn), false, false),
    ("process.kill", stable(PROCESS_JOBS_MINOR), cap(Capability::ProcessSpawn), false, false),
    ("process.open", stable(PROCESS_JOBS_MINOR), cap(Capability::ProcessSpawn), false, false),
    ("process.poll", stable(PROCESS_POLL_MINOR), cap(Capability::ProcessSpawn), false, false),
    ("process.signal", stable(PROCESS_JOBS_MINOR), cap(Capability::ProcessSpawn), false, false),
    ("process.status", stable(PROCESS_JOBS_MINOR), cap(Capability::ProcessSpawn), false, false),
    ("process.write", stable(PROCESS_JOBS_MINOR), cap(Capability::ProcessSpawn), false, false),
    ("task.cancel", experimental(WASM_TASKS_MINOR), cap(Capability::TaskRun), false, false),
    ("task.list", experimental(WASM_TASKS_MINOR), cap(Capability::TaskRun), false, false),
    ("task.poll", experimental(WASM_TASKS_MINOR), cap(Capability::TaskRun), false, false),
    ("task.start", experimental(WASM_TASKS_MINOR), cap(Capability::TaskRun), false, false),
    ("task.status", experimental(WASM_TASKS_MINOR), cap(Capability::TaskRun), false, false),
    ("timer.after", experimental(WASM_AUTOMATION_MINOR), cap(Capability::EditorSchedule), false, false),
    ("timer.cancel", experimental(WASM_AUTOMATION_MINOR), cap(Capability::EditorSchedule), false, false),
    ("timer.every", experimental(WASM_AUTOMATION_MINOR), cap(Capability::EditorSchedule), false, false),
    ("timer.list", experimental(WASM_AUTOMATION_MINOR), cap(Capability::EditorSchedule), false, false),
    ("ui.confirm", stable(INTERACTIVE_UI_MINOR), cap(Capability::UiPrompt), true, true),
    ("ui.input", stable(INTERACTIVE_UI_MINOR), cap(Capability::UiPrompt), true, true),
    ("ui.message", stable(LEGACY_MINOR), cap(Capability::UiMessage), false, true),
    ("ui.overlay_close", experimental(WASM_OVERLAYS_MINOR), cap(Capability::UiOverlay), false, true),
    ("ui.overlay_open", experimental(WASM_OVERLAYS_MINOR), cap(Capability::UiOverlay), false, true),
    ("ui.overlay_update", experimental(WASM_OVERLAYS_MINOR), cap(Capability::UiOverlay), false, true),
    ("ui.overlay_wait", experimental(WASM_OVERLAYS_MINOR), cap(Capability::UiOverlay), true, true),
    ("ui.panel_close", stable(PANELS_MINOR), cap(Capability::UiPanel), false, true),
    ("ui.panel_focus", stable(PANELS_MINOR), cap(Capability::UiPanel), false, true),
    ("ui.panel_open", stable(PANELS_MINOR), cap(Capability::UiPanel), false, true),
    ("ui.panel_update", stable(PANELS_MINOR), cap(Capability::UiPanel), false, true),
    ("ui.picker", experimental(WASM_OVERLAYS_MINOR), cap(Capability::UiPicker), true, true),
    ("ui.progress_finish", stable(INTERACTIVE_UI_MINOR), cap(Capability::UiProgress), false, true),
    ("ui.progress_start", stable(INTERACTIVE_UI_MINOR), cap(Capability::UiProgress), false, true),
    ("ui.progress_update", stable(INTERACTIVE_UI_MINOR), cap(Capability::UiProgress), false, true),
    ("ui.select", stable(INTERACTIVE_UI_MINOR), cap(Capability::UiPrompt), true, true),
    ("ui.set_statusline", stable(DECORATIONS_MINOR), cap(Capability::UiStatusline), false, true),
    ("variable.delete", experimental(WASM_AUTOMATION_MINOR), cap(Capability::EditorVariables), false, false),
    ("variable.get", experimental(WASM_AUTOMATION_MINOR), cap(Capability::EditorVariables), false, false),
    ("variable.list", experimental(WASM_AUTOMATION_MINOR), cap(Capability::EditorVariables), false, false),
    ("variable.set", experimental(WASM_AUTOMATION_MINOR), cap(Capability::EditorVariables), false, false),
    ("window.close", stable(WASM_EDITOR_MINOR), cap(Capability::EditorWindows), false, true),
    ("window.focus", stable(WASM_EDITOR_MINOR), cap(Capability::EditorWindows), false, true),
    ("window.get", stable(WASM_EDITOR_MINOR), cap(Capability::EditorWindows), false, false),
    ("window.list", stable(WASM_EDITOR_MINOR), cap(Capability::EditorWindows), false, false),
    ("window.open", stable(WASM_EDITOR_MINOR), cap(Capability::EditorWindows), false, true),
);

#[rustfmt::skip]
events!(
    ("buffer_changed", stable(EVENTS_MINOR), Capability::EditorRead),
    ("buffer_closed", stable(EVENTS_MINOR), Capability::EditorRead),
    ("buffer_enter", stable(WASM_EDITOR_MINOR), Capability::EditorBuffers),
    ("buffer_leave", stable(WASM_EDITOR_MINOR), Capability::EditorBuffers),
    ("buffer_opened", stable(EVENTS_MINOR), Capability::EditorRead),
    ("cursor_moved", stable(WASM_EDITOR_MINOR), Capability::EditorControl),
    ("diagnostics_changed", stable(EVENTS_MINOR), Capability::DiagnosticsRead),
    ("file_opened", stable(EVENTS_MINOR), Capability::EditorRead),
    ("file_saved", stable(EVENTS_MINOR), Capability::EditorRead),
    ("focus_gained", stable(WASM_EDITOR_MINOR), Capability::EditorWindows),
    ("focus_lost", stable(WASM_EDITOR_MINOR), Capability::EditorWindows),
    ("language_changed", stable(WASM_AUTOCMDS_MINOR), Capability::EditorRead),
    ("mode_changed", stable(EVENTS_MINOR), Capability::EditorRead),
    ("selection_changed", stable(WASM_EDITOR_MINOR), Capability::EditorControl),
    ("user", stable(WASM_AUTOCMDS_MINOR), Capability::EditorAutocmds),
    ("window_closed", stable(WASM_AUTOCMDS_MINOR), Capability::EditorWindows),
    ("window_created", stable(WASM_AUTOCMDS_MINOR), Capability::EditorWindows),
    ("window_enter", stable(WASM_EDITOR_MINOR), Capability::EditorWindows),
    ("window_leave", stable(WASM_EDITOR_MINOR), Capability::EditorWindows),
);
