wit_bindgen::generate!({
    path: "wit",
    world: "guest",
    pub_export_macro: true,
    default_bindings_module: "kawi_plugin_sdk",
});

mod edit;
mod error;
mod offsets;

pub use edit::apply_edit_with_retry;
pub use error::{
    error_kind, ErrorKind, BUFFER_MODIFIED, BUFFER_NOT_ACTIVE, BUFFER_NOT_FOUND, BUFFER_NOT_OWNED,
    CAPABILITY_DENIED, INTERNAL_ERROR, INVALID_PARAMS, INVALID_POSITION, INVALID_REQUEST,
    JOB_CLOSED, JOB_LIMIT, JOB_NOT_FOUND, JOB_QUEUE_FULL, LAST_WINDOW, METHOD_NOT_FOUND,
    MODE_NOT_SUPPORTED, OPTION_NOT_FOUND, OVERLAY_NOT_FOUND, PANEL_NOT_FOUND, PARSE_ERROR,
    PLUGIN_ERROR, PLUGIN_UNAVAILABLE, REQUEST_CANCELED, STALE_REVISION, TASK_LIMIT, TASK_NOT_FOUND,
    TASK_START_FAILED, UI_BUSY, WINDOW_NOT_FOUND,
};
pub use kawi::plugin::types;
pub use kawi::plugin::{
    config_read, diagnostics_publish, editor_autocmds, editor_buffers, editor_buffers_write,
    editor_commands, editor_control, editor_decorations, editor_execute, editor_keymaps,
    editor_read, editor_schedule, editor_signs, editor_variables, editor_windows, editor_write,
    options_read, options_write, process_jobs, process_run, task_runner, ui_message, ui_overlay,
    ui_panel, ui_picker, ui_progress, ui_prompt, ui_statusline,
};
pub use offsets::{byte_offset, position_at};

#[cfg(test)]
mod tests {
    #[test]
    fn wit_package_version_matches_the_crate_version() {
        let wit = include_str!("../wit/plugin.wit");
        let expected = format!("package kawi:plugin@{};", env!("CARGO_PKG_VERSION"));
        assert!(wit.contains(&expected));
    }
}
