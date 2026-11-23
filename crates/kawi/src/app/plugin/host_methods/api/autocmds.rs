use kawi_plugin::{
    validate_user_event, AutocmdRemoveParams, AutocmdSpec, RpcError, UserEventParams,
    INTERNAL_ERROR,
};
use serde_json::Value;

use super::super::super::super::App;
use crate::app::plugin::autocmds::AutocmdError;
use crate::app::plugin::host_methods::util::invalid_params;

pub(super) fn handle(
    app: &mut App,
    plugin: &str,
    method: &str,
    params: Value,
) -> Result<Value, RpcError> {
    match method {
        "autocmd.register" => register(app, plugin, params),
        "autocmd.remove" => remove(app, plugin, params),
        "autocmd.list" => list(app, plugin),
        "autocmd.clear" => clear(app, plugin),
        "editor.emit_user_event" => emit_user_event(app, params),
        _ => Err(RpcError::method_not_found(method)),
    }
}

fn register(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let spec: AutocmdSpec = serde_json::from_value(params).map_err(invalid_params)?;
    let registered = match app.plugins.register_autocmd(plugin, spec) {
        Ok(registered) => registered,
        Err(AutocmdError::Invalid(message)) => return Err(RpcError::invalid_params(message)),
        Err(AutocmdError::Capability(capability)) => {
            return Err(app.plugins.capability_error(plugin, capability));
        }
    };
    serde_json::to_value(registered).map_err(internal_error)
}

fn remove(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: AutocmdRemoveParams = serde_json::from_value(params).map_err(invalid_params)?;
    app.plugins
        .remove_autocmd(plugin, params.id)
        .map_err(RpcError::invalid_params)?;
    Ok(Value::Null)
}

fn list(app: &App, plugin: &str) -> Result<Value, RpcError> {
    let autocmds = app.plugins.registered_autocmds(plugin);
    serde_json::to_value(autocmds).map_err(internal_error)
}

fn clear(app: &mut App, plugin: &str) -> Result<Value, RpcError> {
    app.plugins.clear_autocmds(plugin);
    Ok(Value::Null)
}

fn emit_user_event(app: &mut App, params: Value) -> Result<Value, RpcError> {
    let params: UserEventParams = serde_json::from_value(params).map_err(invalid_params)?;
    validate_user_event(&params).map_err(RpcError::invalid_params)?;
    app.plugins.emit_user_event(params.name, params.payload);
    Ok(Value::Null)
}

fn internal_error(error: serde_json::Error) -> RpcError {
    RpcError::new(INTERNAL_ERROR, error.to_string())
}
