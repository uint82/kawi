use kawi_plugin::{
    validate_command_id, validate_command_spec, validate_keymap_keys, validate_keymap_spec,
    CommandRegisterParams, CommandUnregisterParams, KeymapRemoveParams, KeymapSetParams, RpcError,
};
use serde_json::Value;

use super::super::super::super::App;
use crate::app::plugin::host_methods::util::invalid_params;
use crate::app::plugin::text::sanitize;

pub(super) fn handle(
    app: &mut App,
    plugin: &str,
    method: &str,
    params: Value,
) -> Result<Value, RpcError> {
    match method {
        "command.register" => register(app, plugin, params),
        "command.unregister" => unregister(app, plugin, params),
        "command.list" => list(app, plugin),
        "keymap.set" => set_keymap(app, plugin, params),
        "keymap.remove" => remove_keymap(app, plugin, params),
        "keymap.list" => list_keymaps(app, plugin),
        _ => Err(RpcError::method_not_found(method)),
    }
}

fn register(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let mut params: CommandRegisterParams =
        serde_json::from_value(params).map_err(invalid_params)?;
    params.title = sanitize(&params.title);
    params.description = params.description.map(|description| sanitize(&description));
    validate_command_spec(&params).map_err(RpcError::invalid_params)?;
    let registered = app
        .plugins
        .register_command(plugin, params, &mut app.commands)
        .map_err(RpcError::invalid_params)?;
    serde_json::to_value(registered).map_err(internal_error)
}

fn unregister(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: CommandUnregisterParams = serde_json::from_value(params).map_err(invalid_params)?;
    validate_command_id(&params.id).map_err(RpcError::invalid_params)?;
    app.plugins
        .unregister_command(plugin, &params.id, &mut app.commands, &mut app.keymaps)
        .map_err(RpcError::invalid_params)?;
    Ok(Value::Null)
}

fn list(app: &App, plugin: &str) -> Result<Value, RpcError> {
    let commands = app.plugins.registered_commands(plugin);
    serde_json::to_value(commands).map_err(internal_error)
}

fn set_keymap(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let mut params: KeymapSetParams = serde_json::from_value(params).map_err(invalid_params)?;
    params.description = params.description.map(|description| sanitize(&description));
    validate_keymap_spec(&params).map_err(RpcError::invalid_params)?;
    let registered = app
        .plugins
        .set_keymap(
            plugin,
            params,
            &app.commands,
            &mut app.keymaps,
            &app.leaders,
        )
        .map_err(RpcError::invalid_params)?;
    serde_json::to_value(registered).map_err(internal_error)
}

fn remove_keymap(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: KeymapRemoveParams = serde_json::from_value(params).map_err(invalid_params)?;
    validate_keymap_keys(&params.keys).map_err(RpcError::invalid_params)?;
    app.plugins
        .remove_keymap(plugin, params, &mut app.keymaps, &app.leaders)
        .map_err(RpcError::invalid_params)?;
    Ok(Value::Null)
}

fn list_keymaps(app: &App, plugin: &str) -> Result<Value, RpcError> {
    let keymaps = app.plugins.registered_keymaps(plugin);
    serde_json::to_value(keymaps).map_err(internal_error)
}

fn internal_error(error: serde_json::Error) -> RpcError {
    RpcError::new(kawi_plugin::INTERNAL_ERROR, error.to_string())
}
