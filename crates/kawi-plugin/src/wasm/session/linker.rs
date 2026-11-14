use wasmtime::component::{HasSelf, Linker};

use crate::capability::{Capability, CapabilityRegistry};
use crate::error::PluginError;

use super::super::bindings::kawi::plugin::{
    config_read, diagnostics_publish, editor_buffers, editor_buffers_write, editor_control,
    editor_decorations, editor_execute, editor_read, editor_signs, editor_windows, editor_write,
    options_read, process_jobs, process_run, types, ui_message, ui_panel, ui_progress, ui_prompt,
    ui_statusline,
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
    if capabilities.has(plugin, Capability::EditorExecute) {
        editor_execute::add_to_linker::<WasmState, HasSelf<WasmState>>(linker, |state| state)
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
