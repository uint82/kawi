use kawi_plugin::{
    validate_variable_key, validate_variable_value, RpcError, VariableParams, VariableScope,
    VariableSetParams, VariableTarget, VariableTargetParams,
};
use serde_json::Value;

use super::super::super::super::App;
use super::super::{invalid_params, json_result};

pub(super) fn handle(
    app: &mut App,
    plugin: &str,
    method: &str,
    params: Value,
) -> Result<Value, RpcError> {
    match method {
        "variable.get" => get(app, plugin, params),
        "variable.set" => set(app, plugin, params),
        "variable.delete" => delete(app, plugin, params),
        "variable.list" => list(app, plugin, params),
        _ => Err(RpcError::method_not_found(method)),
    }
}

fn get(app: &App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: VariableParams = serde_json::from_value(params).map_err(invalid_params)?;
    validate_target(app, params.target)?;
    validate_variable_key(&params.key).map_err(RpcError::invalid_params)?;
    json_result(app.plugins.get_variable(plugin, params.target, &params.key))
}

fn set(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: VariableSetParams = serde_json::from_value(params).map_err(invalid_params)?;
    validate_target(app, params.target)?;
    validate_variable_key(&params.key).map_err(RpcError::invalid_params)?;
    validate_variable_value(&params.value).map_err(RpcError::invalid_params)?;
    app.plugins
        .set_variable(plugin, params.target, &params.key, params.value)
        .map_err(RpcError::invalid_params)?;
    Ok(Value::Null)
}

fn delete(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: VariableParams = serde_json::from_value(params).map_err(invalid_params)?;
    validate_target(app, params.target)?;
    validate_variable_key(&params.key).map_err(RpcError::invalid_params)?;
    json_result(
        app.plugins
            .delete_variable(plugin, params.target, &params.key),
    )
}

fn list(app: &App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: VariableTargetParams = serde_json::from_value(params).map_err(invalid_params)?;
    validate_target(app, params.target)?;
    json_result(app.plugins.list_variables(plugin, params))
}

fn validate_target(app: &App, target: VariableTarget) -> Result<(), RpcError> {
    let target = target.validate().map_err(RpcError::invalid_params)?;
    match target.scope {
        VariableScope::Global | VariableScope::Plugin => Ok(()),
        VariableScope::Buffer => {
            let buffer = target.buffer.unwrap_or_default();
            let number = usize::try_from(buffer).map_err(|_| RpcError::buffer_not_found(buffer))?;
            if app.editor.buffer_is_dirty(number).is_none() {
                return Err(RpcError::buffer_not_found(buffer));
            }
            Ok(())
        }
        VariableScope::Window => {
            let window = target.window.unwrap_or_default();
            if app.windows.view(window).is_none() {
                return Err(RpcError::window_not_found(window));
            }
            Ok(())
        }
    }
}
