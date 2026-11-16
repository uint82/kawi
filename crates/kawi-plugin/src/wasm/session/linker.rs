use wasmtime::component::{HasSelf, Linker};

use crate::capability::{Capability, CapabilityRegistry};
use crate::error::PluginError;

use super::super::bindings::kawi::plugin::{
    config_read, diagnostics_publish, editor_autocmds, editor_buffers, editor_buffers_write,
    editor_commands, editor_control, editor_decorations, editor_execute, editor_keymaps,
    editor_read, editor_schedule, editor_signs, editor_variables, editor_windows, editor_write,
    options_read, options_write, process_jobs, process_run, task_runner, types, ui_message,
    ui_overlay, ui_panel, ui_picker, ui_progress, ui_prompt, ui_statusline,
};
use super::super::state::WasmState;

pub(crate) fn add_kawi_interfaces(
    linker: &mut Linker<WasmState>,
    capabilities: &CapabilityRegistry,
    plugin: &str,
) -> Result<(), PluginError> {
    types::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
        .map_err(linker_error)?;
    config_read::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
        .map_err(linker_error)?;
    if capabilities.has(plugin, Capability::EditorRead) {
        editor_read::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorWrite) {
        editor_write::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorBuffers) {
        editor_buffers::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorBuffersWrite) {
        editor_buffers_write::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorWindows) {
        editor_windows::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorControl) {
        editor_control::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::OptionsRead) {
        options_read::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::OptionsWrite) {
        options_write::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorSchedule) {
        editor_schedule::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorVariables) {
        editor_variables::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorExecute) {
        editor_execute::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorCommands) {
        editor_commands::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorKeymaps) {
        editor_keymaps::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorAutocmds) {
        editor_autocmds::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::UiMessage) {
        ui_message::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::UiPrompt) {
        ui_prompt::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::UiProgress) {
        ui_progress::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::DiagnosticsPublish) {
        diagnostics_publish::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::EditorDecorations) {
        editor_decorations::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
        editor_signs::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::UiStatusline) {
        ui_statusline::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::UiPanel) {
        ui_panel::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::UiOverlay) {
        ui_overlay::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::UiPicker) {
        ui_picker::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::TaskRun) {
        task_runner::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    if capabilities.has(plugin, Capability::ProcessSpawn) {
        process_run::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
        process_jobs::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
            .map_err(linker_error)?;
    }
    Ok(())
}

fn linker_error(error: wasmtime::Error) -> PluginError {
    PluginError::Initialize(format!("wasm linker: {error}"))
}
