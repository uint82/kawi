use serde_json::Value;

use kawi_plugin::RpcError;

use super::super::super::App;

mod buffer_api;
mod control_api;
mod options_api;
mod window_api;

pub(super) fn handle(
    app: &mut App,
    plugin: &str,
    method: &str,
    params: Value,
) -> Result<Value, RpcError> {
    match method {
        "buffer.list" => app.plugin_buffer_list(),
        "buffer.get" => app.plugin_buffer_get(params),
        "buffer.open" => app.plugin_buffer_open(plugin, params),
        "buffer.apply_edit" => app.plugin_buffer_apply_edit(plugin, params),
        "window.list" => app.plugin_window_list(),
        "window.get" => app.plugin_window_get(params),
        "window.focus" => app.plugin_window_focus(params),
        "window.open" => app.plugin_window_open(params),
        "window.close" => app.plugin_window_close(params),
        "editor.set_cursor" => app.plugin_set_cursor(params),
        "editor.set_selection" => app.plugin_set_selection(params),
        "editor.set_mode" => app.plugin_set_mode(params),
        "options.list" => app.plugin_options_list(params),
        "options.get" => app.plugin_options_get(params),
        _ => Err(RpcError::method_not_found(method)),
    }
}

pub(super) fn ensure_active_buffer(app: &App, buffer: u64) -> Result<usize, RpcError> {
    let number = usize::try_from(buffer)
        .map_err(|_| RpcError::invalid_params(format!("buffer id out of range: {buffer}")))?;
    if app.editor.buffer_is_dirty(number).is_none() {
        return Err(RpcError::buffer_not_found(buffer));
    }
    if number != app.editor.buffer_number() {
        return Err(RpcError::buffer_not_active(buffer));
    }
    Ok(number)
}
