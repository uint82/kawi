use kawi_plugin::{Capability, RpcError, TaskInfo, TaskJobParams, TaskStartParams};
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
        "task.list" => list(app, plugin),
        "task.start" => start(app, plugin, params),
        "task.status" => status(app, plugin, params),
        "task.poll" => poll(app, plugin, params),
        "task.cancel" => cancel(app, plugin, params),
        _ => Err(RpcError::method_not_found(method)),
    }
}

fn list(app: &App, plugin: &str) -> Result<Value, RpcError> {
    let tasks = app
        .plugins
        .task_names(plugin)
        .into_iter()
        .filter_map(|name| app.tasks.get(&name))
        .map(|spec| TaskInfo {
            name: spec.name.clone(),
            command: spec.command.clone(),
            args: spec.args.clone(),
        })
        .collect::<Vec<_>>();
    json_result(tasks)
}

fn start(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: TaskStartParams = serde_json::from_value(params).map_err(invalid_params)?;
    if !app.plugins.allows_task(plugin, &params.name) {
        return Err(app.plugins.capability_error(plugin, Capability::TaskRun));
    }
    let spec = app
        .tasks
        .get(&params.name)
        .ok_or_else(|| RpcError::task_name_not_found(&params.name))?
        .clone();
    let root = app.project_root();
    let wake = app.events.wake();
    json_result(app.plugins.tasks.start(plugin, &spec, &root, wake)?)
}

fn status(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: TaskJobParams = serde_json::from_value(params).map_err(invalid_params)?;
    json_result(app.plugins.tasks.status(plugin, params.job)?)
}

fn poll(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: TaskJobParams = serde_json::from_value(params).map_err(invalid_params)?;
    json_result(app.plugins.tasks.poll(plugin, params.job)?)
}

fn cancel(app: &mut App, plugin: &str, params: Value) -> Result<Value, RpcError> {
    let params: TaskJobParams = serde_json::from_value(params).map_err(invalid_params)?;
    app.plugins.tasks.cancel(plugin, params.job)?;
    Ok(Value::Null)
}
