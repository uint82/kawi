use std::io::BufReader;
use std::process::ChildStdout;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;

use serde_json::Value;

use super::channel::{fail_pending, send_log, ReaderContext};
use crate::capability::is_process_host_method;
use crate::error::PluginError;
use crate::host::{send_event, PluginHostEvent};
use crate::protocol::{error_response_value, is_interactive_host_method, RpcError};

pub(super) fn spawn_reader(
    stdout: ChildStdout,
    context: Arc<ReaderContext>,
) -> Result<(), PluginError> {
    thread::Builder::new()
        .name("kawi-plugin-reader".to_string())
        .spawn(move || read_loop(stdout, context))
        .map(|_| ())
        .map_err(|error| PluginError::Spawn(error.to_string()))
}

fn read_loop(stdout: ChildStdout, context: Arc<ReaderContext>) {
    let mut reader = BufReader::new(stdout);
    loop {
        match kawi_jsonrpc::framing::read_message(&mut reader) {
            Ok(Some(value)) => {
                if let Err(error) = handle_message(&value, &context) {
                    fail_pending(&context.pending, error);
                    context.alive.store(false, Ordering::SeqCst);
                    return;
                }
            }
            Ok(None) => {
                fail_pending(&context.pending, PluginError::ProcessExited);
                context.alive.store(false, Ordering::SeqCst);
                return;
            }
            Err(error) => {
                let error = PluginError::Protocol(error.to_string());
                send_log(
                    &context.events,
                    &context.wake,
                    &context.plugin,
                    error.to_string(),
                );
                fail_pending(&context.pending, error);
                context.alive.store(false, Ordering::SeqCst);
                return;
            }
        }
    }
}

fn handle_message(value: &Value, context: &ReaderContext) -> Result<(), PluginError> {
    if let Some(method) = value.get("method").and_then(Value::as_str) {
        return handle_request_or_notification(value, method, context);
    }
    handle_response(value, context)
}

fn handle_request_or_notification(
    value: &Value,
    method: &str,
    context: &ReaderContext,
) -> Result<(), PluginError> {
    if !is_process_host_method(method) {
        if let Some(id) = value.get("id").cloned() {
            let error = RpcError::method_not_found(method);
            let _ = context.writer.try_send(error_response_value(id, &error));
        } else {
            send_log(
                &context.events,
                &context.wake,
                &context.plugin,
                format!("notification ignored: {method}"),
            );
        }
        return Ok(());
    }
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    if is_interactive_host_method(method) {
        if let Some(id) = id.as_u64() {
            context.interaction.begin(id);
        }
    }
    let params = value.get("params").cloned().unwrap_or(Value::Null);
    send_event(
        &context.events,
        &context.wake,
        PluginHostEvent::HostRequest {
            plugin: context.plugin.clone(),
            id,
            method: method.to_string(),
            params,
        },
    );
    Ok(())
}

fn handle_response(value: &Value, context: &ReaderContext) -> Result<(), PluginError> {
    let Some(id) = value.get("id").and_then(Value::as_u64) else {
        return Err(PluginError::Protocol(
            "response has no integer id".to_string(),
        ));
    };
    let result = if let Some(result) = value.get("result") {
        Ok(result.clone())
    } else if let Some(error) = value.get("error") {
        Err(PluginError::Rpc(parse_error(error)?))
    } else {
        return Err(PluginError::Protocol(
            "response has no result or error".to_string(),
        ));
    };
    let sender = context
        .pending
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .remove(&id);
    if let Some(sender) = sender {
        let _ = sender.send(result);
    }
    Ok(())
}

fn parse_error(value: &Value) -> Result<RpcError, PluginError> {
    let code = value
        .get("code")
        .and_then(Value::as_i64)
        .ok_or_else(|| PluginError::Protocol("error has no code".to_string()))?;
    let message = value
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| PluginError::Protocol("error has no message".to_string()))?;
    let error = RpcError::new(code, message);
    Ok(match value.get("data") {
        Some(data) => error.with_data(data.clone()),
        None => error,
    })
}
