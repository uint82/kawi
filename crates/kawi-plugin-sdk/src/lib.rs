wit_bindgen::generate!({
    path: "wit",
    world: "host",
    pub_export_macro: true,
    default_bindings_module: "kawi_plugin_sdk",
});

mod edit;
mod error;
mod offsets;

pub use edit::apply_edit_with_retry;
pub use error::{
    error_kind, ErrorKind, CAPABILITY_DENIED, INTERNAL_ERROR, INVALID_PARAMS, INVALID_POSITION,
    INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, PLUGIN_ERROR, PLUGIN_UNAVAILABLE,
    REQUEST_CANCELED, STALE_REVISION, UI_BUSY,
};
pub use kawi::plugin::types;
pub use kawi::plugin::{
    config_read, diagnostics_publish, editor_decorations, editor_execute, editor_read,
    editor_signs, editor_write, process_jobs, process_run, ui_message, ui_progress, ui_prompt,
    ui_statusline,
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
