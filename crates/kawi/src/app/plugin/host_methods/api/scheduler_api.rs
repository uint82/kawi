use std::time::Instant;

use kawi_plugin::{RpcError, TimerAfterParams, TimerCancelParams, TimerEveryParams};
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
        "timer.after" => after(app, plugin, params),
        "timer.every" => every(app, plugin, params),
        "timer.cancel" => cancel(app, plugin, params),
        "timer.list" => list(app, plugin),
        _ => Err(RpcError::method_not_found(method)),
    }
}

fn after(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: TimerAfterParams = serde_json::from_value(params).map_err(invalid_params)?;
    let id = app
        .plugins
        .register_after_timer(plugin, params, Instant::now())
        .map_err(RpcError::invalid_params)?;
    json_result(id)
}

fn every(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: TimerEveryParams = serde_json::from_value(params).map_err(invalid_params)?;
    let id = app
        .plugins
        .register_every_timer(plugin, params, Instant::now())
        .map_err(RpcError::invalid_params)?;
    json_result(id)
}

fn cancel(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: TimerCancelParams = serde_json::from_value(params).map_err(invalid_params)?;
    json_result(app.plugins.cancel_timer(plugin, params.timer))
}

fn list(app: &App, plugin: &str) -> Result<Value, RpcError> {
    json_result(app.plugins.registered_timers(plugin))
}
